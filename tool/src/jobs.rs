#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::scena::Title;
use crate::{dt, itp, scena, scena_json};

pub const MANIFEST: &str = ".kiseki_manifest.tsv";

static PALETTE_SKIPPED: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug)]
pub struct Options {
	pub title: Title,
	pub game_dir: PathBuf,
	pub proj_dir: PathBuf,
	pub out_dir: PathBuf,
	pub scena: bool,
	pub tables: bool,
	pub textures: bool,
	pub other: bool,
	pub tex_dds: bool,
	pub check: bool,
	pub mask: String,
	pub only_changed: bool,
	pub overwrite_edited: bool,
	pub include: Option<Vec<String>>,
	pub build_only: Option<Vec<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
	Scena,
	Table,
	Texture,
	Other,
	Sprite,
}

const RAW: &str = "raw";

impl Kind {
	fn label(self) -> &'static str {
		match self {
			Kind::Scena => tr("диалоги", "dialogues"),
			Kind::Table => tr("таблицы", "tables"),
			Kind::Texture => tr("текстуры", "textures"),
			Kind::Other => tr("прочие файлы (как есть)", "other files (as they are)"),
			Kind::Sprite => tr("спрайты .itc", ".itc sprites"),
		}
	}
}

pub trait Sink: Sync {
	fn log(&self, s: &str);
	fn progress(&self, done: usize, total: usize);
	fn cancelled(&self) -> bool { false }
	fn current(&self, _name: &str) {}
}

#[derive(Default, Debug)]
pub struct Summary {
	pub ok: usize,
	pub skipped: usize,
	pub failed: usize,
	pub warnings: usize,
	pub check: Vec<String>,
}

fn fnv(data: &[u8]) -> u64 {
	let mut h: u64 = 0xcbf29ce484222325;
	for &b in data {
		h ^= b as u64;
		h = h.wrapping_mul(0x100000001b3);
	}
	h
}

fn wildcard(pat: &[char], s: &[char]) -> bool {
	match (pat.first(), s.first()) {
		(None, None) => true,
		(Some('*'), _) => wildcard(&pat[1..], s) || (!s.is_empty() && wildcard(pat, &s[1..])),
		(Some('?'), Some(_)) => wildcard(&pat[1..], &s[1..]),
		(Some(a), Some(b)) if a == b => wildcard(&pat[1..], &s[1..]),
		_ => false,
	}
}

pub fn mask_match(mask: &str, stem: &str) -> bool {
	let pats: Vec<&str> = mask.split(|c: char| c == ',' || c == ';' || c.is_whitespace()).filter(|p| !p.is_empty()).collect();
	if pats.is_empty() {
		return true;
	}
	let s: Vec<char> = stem.to_lowercase().chars().collect();
	pats.iter().any(|p| {
		let p = p.to_lowercase();
		let full: Vec<char> = p.chars().collect();
		let base = p.split('.').next().unwrap_or(&p).to_string();
		let bc: Vec<char> = base.chars().collect();
		wildcard(&full, &s) || (p.contains('.') && bc.iter().any(|&c| c != '*' && c != '?') && wildcard(&bc, &s))
	})
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
	let Ok(rd) = std::fs::read_dir(dir) else { return };
	let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
	entries.sort();
	for p in entries {
		if p.is_dir() {
			let n = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
			if n.starts_with("_backup") { continue }
			walk(&p, out);
		} else {
			out.push(p);
		}
	}
}

fn file_name(p: &Path) -> String {
	p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn stem_of(name: &str) -> String {
	name.split('.').next().unwrap_or(name).to_string()
}

fn is_in_scena_dir(rel: &Path, base: &Path) -> bool {
	let base_is = base.file_name().map(|n| n.to_string_lossy().to_lowercase().starts_with("scena")).unwrap_or(false);
	base_is || rel.parent().map(|p| p.components().any(|c| c.as_os_str().to_string_lossy().to_lowercase().starts_with("scena"))).unwrap_or(false)
}

fn data_rel(f: &Path, game_dir: &Path) -> PathBuf {
	let rel = f.strip_prefix(game_dir).unwrap_or(f).to_path_buf();
	let comps: Vec<_> = rel.components().collect();
	if comps.len() >= 2 {
		if let Some(pos) = comps[..comps.len() - 1].iter().rposition(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case("data")) {
			return comps[pos + 1..].iter().collect();
		}
	}
	if rel.parent().map(|p| p.as_os_str().is_empty()).unwrap_or(true) {
		if let Some(n) = game_dir.file_name() {
			return Path::new(n).join(rel);
		}
	}
	rel
}

fn game_file(o: &Options, rel: &Path) -> Option<PathBuf> {
	let g = &o.game_dir;
	if g.as_os_str().is_empty() {
		return None;
	}
	let mut c = vec![g.join(rel), g.join("data").join(rel)];
	if let (Some(first), Some(parent)) = (rel.components().next(), g.parent()) {
		if g.file_name().map(|n| n.eq_ignore_ascii_case(first.as_os_str())).unwrap_or(false) {
			c.push(parent.join(rel));
		}
	}
	c.into_iter().find(|p| p.is_file())
}

fn read_text(path: &Path) -> Result<String, String> {
	let b = std::fs::read(path).map_err(|e| lformat!("не удалось прочитать: {e}", "could not read: {e}"))?;
	let b = b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&b);
	String::from_utf8(b.to_vec()).map_err(|_| tr("файл не в кодировке UTF-8 (в Notepad++: Кодировки → Преобразовать в UTF-8)", "the file is not UTF-8 (in Notepad++: Encoding → Convert to UTF-8)").to_string())
}

#[derive(Default)]
struct Manifest {
	map: BTreeMap<String, (u64, String)>,
}

impl Manifest {
	fn load(dir: &Path) -> Manifest {
		let mut m = Manifest::default();
		if let Ok(s) = std::fs::read_to_string(dir.join(MANIFEST)) {
			for line in s.lines() {
				let mut it = line.split('\t');
				if let (Some(p), Some(h)) = (it.next(), it.next()) {
					if let Ok(h) = u64::from_str_radix(h, 16) {
						m.map.insert(p.to_string(), (h, it.next().unwrap_or("").to_string()));
					}
				}
			}
		}
		m
	}
	fn save(&self, dir: &Path) {
		let mut s = String::from("");
		for (p, (h, meta)) in &self.map {
			s.push_str(&format!("{p}\t{h:016x}\t{meta}\n"));
		}
		let _ = write_atomic(&dir.join(MANIFEST), s.as_bytes());
	}
}

fn rel_key(rel: &Path) -> String {
	rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

fn meta_get(meta: &str, key: &str) -> Option<usize> {
	meta.split(';').find_map(|kv| kv.strip_prefix(key)?.strip_prefix('=')?.parse().ok())
}

struct Task {
	kind: Kind,
	src: PathBuf,
	rel: PathBuf,
}

fn run_parallel<F>(tasks: Vec<Task>, sink: &dyn Sink, f: F) -> Summary
where
	F: Fn(&Task) -> Outcome + Sync,
{
	let total = tasks.len();
	let next = AtomicUsize::new(0);
	let done = AtomicUsize::new(0);
	let summary = Mutex::new(Summary::default());
	let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 8);
	sink.progress(0, total);
	std::thread::scope(|s| {
		for _ in 0..threads {
			s.spawn(|| loop {
				if sink.cancelled() { break }
				let i = next.fetch_add(1, Ordering::SeqCst);
				let Some(t) = tasks.get(i) else { break };
				let name = rel_key(&t.rel);
				sink.current(&name);
				let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(t)))
					.unwrap_or_else(|_| Outcome::Fail(tr("внутренняя ошибка программы (panic)", "internal program error (panic)").into()));
				{
					let mut sm = summary.lock().unwrap();
					match &out {
						Outcome::Ok(None) => sm.ok += 1,
						Outcome::Ok(Some(w)) => { sm.ok += 1; sm.warnings += 1; sink.log(&format!("⚠ {name}: {w}")) }
						Outcome::Skip(Some(r)) => { sm.skipped += 1; sink.log(&format!("— {name}: {r}")) }
						Outcome::Skip(None) => sm.skipped += 1,
						Outcome::Fail(e) => { sm.failed += 1; sink.log(&format!("✖ {name}: {e}")) }
					}
				}
				let d = done.fetch_add(1, Ordering::SeqCst) + 1;
				sink.progress(d, total);
			});
		}
	});
	summary.into_inner().unwrap()
}

enum Outcome {
	Ok(Option<String>),
	Skip(Option<String>),
	Fail(String),
}

fn write_atomic(dst: &Path, data: &[u8]) -> std::io::Result<()> {
	let mut name = dst.file_name().map(|n| n.to_os_string()).unwrap_or_default();
	name.push(".part");
	let tmp = dst.with_file_name(name);
	if let Err(e) = std::fs::write(&tmp, data) {
		let _ = std::fs::remove_file(&tmp);
		return Err(e);
	}
	std::fs::rename(&tmp, dst).inspect_err(|_| {
		let _ = std::fs::remove_file(&tmp);
	})
}

fn kinds_enabled(o: &Options) -> Vec<Kind> {
	let mut v = Vec::new();
	if o.scena { v.push(Kind::Scena) }
	if o.tables { v.push(Kind::Table) }
	if o.textures { v.push(Kind::Texture) }
	if o.other { v.push(Kind::Other); v.push(Kind::Sprite) }
	v
}

fn report(sink: &dyn Sink, what: &str, s: &Summary) {
	sink.log(&lformat!("Готово ({what}): успешно {}, пропущено {}, ошибок {}{}", "Done ({what}): {} ok, {} skipped, {} errors{}",
		s.ok,
		s.skipped,
		s.failed,
		if s.warnings > 0 { lformat!(", предупреждений {}", ", {} warnings", s.warnings) } else { String::new() }
	));
}

pub fn decompile(o: &Options, sink: &dyn Sink) -> Summary {
	let kinds = kinds_enabled(o);
	if kinds.is_empty() {
		sink.log(tr("Не выбран ни один тип файлов.", "No file type selected."));
		return Summary::default();
	}
	if !o.game_dir.is_dir() {
		sink.log(&lformat!("Папка игры не найдена: {}", "Game folder not found: {}", o.game_dir.display()));
		return Summary::default();
	}
	if let Err(e) = std::fs::create_dir_all(&o.proj_dir) {
		sink.log(&lformat!("Не удалось создать папку проекта: {e}", "Could not create the project folder: {e}"));
		return Summary::default();
	}

	let mut files = Vec::new();
	walk(&o.game_dir, &mut files);
	if let Some(inc) = &o.include {
		files.retain(|f| top_dir(f, &o.game_dir).map(|t| inc.contains(&t)).unwrap_or(o.other));
	}
	let data_dir = Some(o.game_dir.join("data")).filter(|d| d.is_dir());
	let mut tasks = Vec::new();
	let mut unsupported = Vec::new();
	for f in files {
		let name = file_name(&f);
		let lname = name.to_lowercase();
		let rel = data_rel(&f, &o.game_dir);
		let rel = rel.as_path();
		let stem = stem_of(&name);
		let kind = if lname.ends_with(".bin") && is_in_scena_dir(rel, &o.game_dir) {
			Kind::Scena
		} else if lname.ends_with("._dt") || lname.ends_with(".dt2") {
			Kind::Table
		} else if lname.ends_with(".itp") {
			Kind::Texture
		} else if data_dir.as_ref().is_none_or(|d| f.starts_with(d)) {
			if lname.ends_with(".itc") { Kind::Sprite } else { Kind::Other }
		} else {
			continue;
		};
		if !kinds.contains(&kind) || !mask_match(&o.mask, &stem) {
			continue;
		}
		if kind == Kind::Table && !dt::is_supported(&stem) {
			unsupported.push(name);
			tasks.push(Task { kind: Kind::Other, src: f.clone(), rel: rel.to_path_buf() });
			continue;
		}
		let out_rel = match kind {
			Kind::Other | Kind::Sprite => rel.to_path_buf(),
			Kind::Scena => rel.with_file_name(format!("{stem}.json")),
			Kind::Table => rel.with_file_name(format!("{stem}.xlsx")),
			Kind::Texture if o.tex_dds => rel.with_file_name(format!("{stem}.dds")),
			Kind::Texture => rel.with_file_name(format!("{stem}.png")),
		};
		tasks.push(Task { kind, src: f.clone(), rel: out_rel });
	}

	let counts = |k: Kind| tasks.iter().filter(|t| t.kind == k).count();
	sink.log(&lformat!("Декомпиляция ({}): {} → {}", "Decompiling ({}): {} → {}",
		o.title.label(),
		o.game_dir.display(),
		o.proj_dir.display()
	));
	if let Some(inc) = &o.include {
		sink.log(&lformat!("  выбрано: {}", "  selected: {}", inc.join(", ")));
	}
	for k in &kinds {
		sink.log(&lformat!("  {}: {} файлов", "  {}: {} files", k.label(), counts(*k)));
	}
	if !unsupported.is_empty() {
		sink.log(&lformat!("  таблицы без поддержки (в проект как есть): {}", "  unsupported tables (copied to the project as is): {}", unsupported.join(", ")));
	}
	if tasks.is_empty() {
		sink.log(tr("Подходящих файлов не найдено. Укажите папку data игры (в ней scena_us, text_us, visual_us…).", "No suitable files found. Choose the game's data folder (it holds scena_us, text_us, visual_us…)."));
		return Summary::default();
	}

	let manifest = Mutex::new(Manifest::load(&o.proj_dir));
	let summary = run_parallel(tasks, sink, |t| {
		let dst = o.proj_dir.join(&t.rel);
		let key = rel_key(&t.rel);
		let edited = |dst: &Path, key: &str| -> Option<Outcome> {
			if !dst.exists() || o.overwrite_edited {
				return None;
			}
			let known = manifest.lock().unwrap().map.get(key).map(|a| a.0);
			let cur = std::fs::read(dst).map(|d| fnv(&d)).ok();
			(known != cur).then(|| Outcome::Skip(Some(tr("уже есть и изменён вами — не перезаписан (чтобы перезаписать, включите «Декомпиляция может перезаписать мои правки»)", "already exists and was edited by you: not overwritten (to overwrite, enable 'Decompile may overwrite my edits')").into())))
		};
		if let Some(r) = edited(&dst, &key) {
			return r;
		}
		let data = match std::fs::read(&t.src) {
			Ok(d) => d,
			Err(e) => return Outcome::Fail(lformat!("не удалось прочитать: {e}", "could not read: {e}")),
		};
		if let Some(p) = dst.parent() {
			let _ = std::fs::create_dir_all(p);
		}
		let as_is = |warn: Option<String>| -> Outcome {
			let raw = t.rel.with_file_name(file_name(&t.src));
			let (dst, key) = (o.proj_dir.join(&raw), rel_key(&raw));
			if let Some(r) = edited(&dst, &key) {
				return r;
			}
			if let Err(e) = write_atomic(&dst, &data) {
				return Outcome::Fail(lformat!("не удалось записать: {e}", "could not write: {e}"));
			}
			manifest.lock().unwrap().map.insert(key, (fnv(&data), RAW.into()));
			Outcome::Ok(warn)
		};
		let (bytes_written, meta, warn): (Vec<u8>, String, Option<String>) = match t.kind {
			Kind::Scena => match scena::decompile_json(o.title, &data) {
				Ok((text, warn)) => {
					let bytes = text.replace("\r\n", "\n").replace('\n', "\r\n").into_bytes();
					if let Err(e) = write_atomic(&dst, &bytes) { return Outcome::Fail(e.to_string()) }
					(bytes, "scena".into(), warn)
				}
				Err(e) => return Outcome::Fail(e),
			},
			Kind::Table => match dt::decompile_xlsx(&data, &stem_of(&file_name(&t.src)), &file_name(&t.src)) {
				Ok((book, _n)) => {
					if let Err(e) = write_atomic(&dst, &book) {
						return Outcome::Fail(lformat!("не удалось записать (файл открыт в Excel?): {e}", "could not write (is the file open in Excel?): {e}"));
					}
					(book, "table".into(), None)
				}
				Err(e) => return as_is(Some(lformat!("таблица не разобралась, в проекте как есть: {e}", "couldn't decompile the table, it stays in the project as is: {e}"))),
			},
			Kind::Texture => match if o.tex_dds { itp::to_dds(&data) } else { itp::decompile(&data) } {
				Ok(dec) => {
					if let Err(e) = write_atomic(&dst, &dec.png) { return Outcome::Fail(e.to_string()) }
					let old = if dec.old { ";old" } else { "" };
					(dec.png, format!("itp;w={};h={};mips={}{old}", dec.width, dec.height, dec.mips), None)
				}
				Err(e) if e.contains(tr("палитр", "palette")) => {
					PALETTE_SKIPPED.fetch_add(1, Ordering::Relaxed);
					return as_is(None);
				}
				Err(e) => return as_is(Some(lformat!("текстура не разобралась, в проекте как есть: {e}", "couldn't decompile the texture, it stays in the project as is: {e}"))),
			},
			Kind::Other => return as_is(None),
			Kind::Sprite => {
				let frames = match itp::itc_decode(&data) {
					Ok(f) => f,
					Err(e) => return as_is(Some(lformat!("спрайт не разобрался, в проекте как есть: {e}", "couldn't decompile the sprite, it stays in the project as is: {e}"))),
				};
				let names: Vec<(PathBuf, String)> = (0..frames.len()).map(|i| { let r = t.rel.join(format!("{i:03}.png")); (o.proj_dir.join(&r), rel_key(&r)) }).collect();
				for (p, k) in &names {
					if let Some(r) = edited(p, k) {
						return r;
					}
				}
				if dst.is_file() && !o.overwrite_edited {
					let same = manifest.lock().unwrap().map.get(&key).map(|a| a.0) == std::fs::read(&dst).map(|d| fnv(&d)).ok();
					if !same {
						return Outcome::Skip(Some(tr("уже есть и изменён вами — не перезаписан (чтобы перезаписать, включите «Декомпиляция может перезаписать мои правки»)", "already exists and was edited by you: not overwritten (to overwrite, enable 'Decompile may overwrite my edits')").into()));
					}
				}
				if dst.is_file() {
					let _ = std::fs::remove_file(&dst);
					manifest.lock().unwrap().map.remove(&key);
				}
				if let Err(e) = std::fs::create_dir_all(&dst) {
					return Outcome::Fail(lformat!("не удалось создать папку: {e}", "can't create the folder: {e}"));
				}
				for (i, (png, (p, k))) in frames.iter().zip(&names).enumerate() {
					if let Err(e) = write_atomic(p, png) {
						return Outcome::Fail(lformat!("не удалось записать: {e}", "could not write: {e}"));
					}
					manifest.lock().unwrap().map.insert(k.clone(), (fnv(png), format!("sprite;i={i}")));
				}
				return Outcome::Ok(None);
			}
		};
		manifest.lock().unwrap().map.insert(key, (fnv(&bytes_written), meta));
		Outcome::Ok(warn)
	});
	manifest.into_inner().unwrap().save(&o.proj_dir);
	let pal = PALETTE_SKIPPED.swap(0, Ordering::Relaxed);
	if pal > 0 {
		sink.log(&lformat!("  текстуры старого формата (с палитрой) — в проект как есть: {pal}; это модели, карты и монстры, текста в них нет", "  old format textures (with a palette), copied to the project as is: {pal}; these are models, maps and monsters, with no text"));
	}
	report(sink, tr("декомпиляция", "decompile"), &summary);
	summary
}

pub fn compile(o: &Options, sink: &dyn Sink) -> Summary {
	let kinds = kinds_enabled(o);
	if kinds.is_empty() {
		sink.log(tr("Не выбран ни один тип файлов.", "No file type selected."));
		return Summary::default();
	}
	if !o.proj_dir.is_dir() {
		sink.log(&lformat!("Папка проекта не найдена: {}", "Project folder not found: {}", o.proj_dir.display()));
		return Summary::default();
	}
	let manifest = Manifest::load(&o.proj_dir);

	let mut files = Vec::new();
	walk(&o.proj_dir, &mut files);
	let mut tasks = Vec::new();
	let mut unchanged = 0usize;
	let mut sprites: BTreeMap<PathBuf, ()> = BTreeMap::new();
	for f in files {
		let name = file_name(&f);
		let lname = name.to_lowercase();
		let Ok(rel) = f.strip_prefix(&o.proj_dir) else { continue };
		let stem = stem_of(&name);
		if name.starts_with("~$") || name.starts_with('.') {
			continue;
		}
		if let Some(only) = &o.build_only {
			if !top_dir(&f, &o.proj_dir).map(|t| only.contains(&t)).unwrap_or(false) {
				continue;
			}
		}
		if let Some(dir) = rel.parent().filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("itc"))) {
			if mask_match(&o.mask, &stem_of(&file_name(dir))) && manifest.map.get(&rel_key(rel)).is_some_and(|(h, _)| std::fs::read(&f).map(|d| fnv(&d)).ok() != Some(*h)) {
				sprites.insert(dir.to_path_buf(), ());
			}
			continue;
		}
		if let Some((h, meta)) = manifest.map.get(&rel_key(rel)) {
			if meta == RAW {
				if mask_match(&o.mask, &stem) && std::fs::read(&f).map(|d| fnv(&d)).ok() != Some(*h) {
					tasks.push(Task { kind: Kind::Other, src: f.clone(), rel: rel.to_path_buf() });
				}
				continue;
			}
			if meta.ends_with(";old") && std::fs::read(&f).map(|d| fnv(&d)).ok() == Some(*h) {
				continue;
			}
		}
		let kind = if lname.ends_with(".png") || lname.ends_with(".dds") {
			Kind::Texture
		} else if lname.ends_with(".xlsx") {
			Kind::Table
		} else if lname.ends_with(".json") {
			let head = std::fs::read(&f).map(|d| String::from_utf8_lossy(&d[..d.len().min(300)]).into_owned()).unwrap_or_default();
			if scena_json::looks_like(&head) { Kind::Scena } else { continue }
		} else if lname.ends_with(".txt") {
			let head = std::fs::read(&f).map(|d| String::from_utf8_lossy(&d[..d.len().min(200)]).into_owned()).unwrap_or_default();
			if scena::looks_like_scena_text(&head) {
				Kind::Scena
			} else if dt::looks_like_table_text(&head) {
				Kind::Table
			} else {
				continue;
			}
		} else {
			continue;
		};
		if !kinds.contains(&kind) || !mask_match(&o.mask, &stem) {
			continue;
		}
		let key = rel_key(rel);
		if o.only_changed {
			if let Some((h, _)) = manifest.map.get(&key) {
				if std::fs::read(&f).map(|d| fnv(&d)).ok() == Some(*h) {
					unchanged += 1;
					continue;
				}
			}
		}
		let ext = match kind {
			Kind::Scena => "bin",
			Kind::Table if stem.to_lowercase().starts_with("monsnote") => "dt2",
			Kind::Table => "_dt",
			Kind::Texture => "itp",
			Kind::Other | Kind::Sprite => continue,
		};
		let out_rel = rel.with_file_name(format!("{stem}.{ext}"));
		tasks.push(Task { kind, src: f.clone(), rel: out_rel });
	}

	for dir in sprites.into_keys() {
		tasks.push(Task { kind: Kind::Sprite, src: o.proj_dir.join(&dir), rel: dir });
	}
	sink.log(&lformat!("Сборка: {} → {}", "Building: {} → {}", o.proj_dir.display(), o.out_dir.display()));
	if let Some(only) = &o.build_only {
		sink.log(&lformat!("  выбрано: {}", "  selected: {}", only.join(", ")));
	}
	for k in &kinds {
		sink.log(&lformat!("  {}: {} файлов", "  {}: {} files", k.label(), tasks.iter().filter(|t| t.kind == *k).count()));
	}
	if o.only_changed && unchanged > 0 {
		sink.log(&lformat!("  без изменений (пропущены): {unchanged}", "  unchanged (skipped): {unchanged}"));
	}
	if tasks.is_empty() {
		sink.log(tr("Нечего собирать.", "Nothing to build."));
		return Summary::default();
	}
	if o.check {
		crate::kai::compare::set_ed7(Some(o.title));
		let list: Vec<(String, PathBuf, PathBuf)> = tasks
			.iter()
			.filter(|t| matches!(t.kind, Kind::Scena | Kind::Table))
			.filter_map(|t| game_file(o, &t.rel).map(|en| (rel_key(t.src.strip_prefix(&o.proj_dir).unwrap_or(&t.src)), en, t.src.clone())))
			.collect();
		let found = if list.is_empty() { Vec::new() } else { crate::kai::compare::check_files(&list) };
		if !found.is_empty() {
			crate::kai::jobs::log_check(sink, &found);
			return Summary { check: found, ..Summary::default() };
		}
	}
	if let Err(e) = std::fs::create_dir_all(&o.out_dir) {
		sink.log(&lformat!("Не удалось создать папку для результата: {e}", "Could not create the output folder: {e}"));
		return Summary::default();
	}

	let summary = run_parallel(tasks, sink, |t| {
		let dst = o.out_dir.join(&t.rel);
		if let Some(p) = dst.parent() {
			let _ = std::fs::create_dir_all(p);
		}
		let fname = file_name(&t.src);
		let (data, warn) = match t.kind {
			Kind::Scena => {
				let src = match read_text(&t.src) { Ok(s) => s, Err(e) => return Outcome::Fail(e) };
				match scena::compile_any(&src, &fname) {
					Ok((d, w)) => (d, if w.is_empty() { None } else { Some(w.join("")) }),
					Err(e) => return Outcome::Fail(e),
				}
			}
			Kind::Table if fname.to_lowercase().ends_with(".xlsx") => {
				let book = match std::fs::read(&t.src) { Ok(d) => d, Err(e) => return Outcome::Fail(lformat!("не удалось прочитать (закройте файл в Excel?): {e}", "could not read (close the file in Excel?): {e}")) };
				let orig = game_file(o, &t.rel).and_then(|p| std::fs::read(p).ok());
				let res = match &orig {
					Some(or) => dt::compile_xlsx_checked(&book, or, &stem_of(&file_name(&t.rel))),
					None => dt::compile_xlsx(&book).map(|d| (d, Vec::new())),
				};
				match res {
					Ok((d, fixed)) if fixed.is_empty() => (d, None),
					Ok((d, fixed)) => (d, Some(lformat!("служебное переводить нельзя — оставлено как в оригинале: {}", "service fields must not be translated, left as in the original: {}", fixed.join("; ")))),
					Err(e) => return Outcome::Fail(e),
				}
			}
			Kind::Table => {
				let src = match read_text(&t.src) { Ok(s) => s, Err(e) => return Outcome::Fail(e) };
				match dt::compile(&src) {
					Ok(d) => (d, None),
					Err(e) => return Outcome::Fail(e),
				}
			}
			Kind::Texture => {
				let png = match std::fs::read(&t.src) { Ok(d) => d, Err(e) => return Outcome::Fail(e.to_string()) };
				let key = rel_key(&t.src.strip_prefix(&o.proj_dir).unwrap_or(&t.src));
				let meta = manifest.map.get(&key).map(|a| a.1.clone()).unwrap_or_default();
				let mips = meta_get(&meta, "mips").unwrap_or(0);
				let expect = meta_get(&meta, "w").zip(meta_get(&meta, "h"));
				let res = if t.src.extension().is_some_and(|e| e.eq_ignore_ascii_case("dds")) { itp::compile_dds(&png, mips, expect) } else { itp::compile(&png, mips, expect) };
				match res {
					Ok(r) => r,
					Err(e) => return Outcome::Fail(e),
				}
			}
			Kind::Other => match std::fs::read(&t.src) {
				Ok(d) => (d, None),
				Err(e) => return Outcome::Fail(lformat!("не удалось прочитать: {e}", "could not read: {e}")),
			},
			Kind::Sprite => {
				let Some(orig) = game_file(o, &t.rel).and_then(|p| std::fs::read(p).ok()) else {
					return Outcome::Fail(tr("нужен файл игры: укажите папку игры, из него берётся всё, кроме изменённых кадров", "the game file is needed: set the game folder, everything except the changed frames comes from it").into());
				};
				let mut edited = Vec::new();
				for (key, (h, meta)) in manifest.map.range(rel_key(&t.rel) + "/"..) {
					if !key.starts_with(&(rel_key(&t.rel) + "/")) {
						break;
					}
					let Some(i) = meta_get(meta, "i") else { continue };
					let png = match std::fs::read(o.proj_dir.join(key)) {
						Ok(d) => d,
						Err(e) => return Outcome::Fail(lformat!("не удалось прочитать кадр {key}: {e}", "can't read frame {key}: {e}")),
					};
					if fnv(&png) != *h {
						edited.push((i, png));
					}
				}
				match itp::itc_replace(&orig, &edited) {
					Ok(d) => (d, None),
					Err(e) => return Outcome::Fail(e),
				}
			}
		};
		if let Err(e) = write_atomic(&dst, &data) {
			return Outcome::Fail(lformat!("не удалось записать {}: {e}", "could not write {}: {e}", dst.display()));
		}
		if std::fs::read(&dst).ok().as_deref() != Some(&data[..]) {
			return Outcome::Fail(tr("файл записан с ошибкой — соберите заново; если повторится, проверьте диск", "the file was written with an error: build again; if it repeats, check the disk").into());
		}
		let out_name = file_name(&dst);
		let back = match t.kind {
			Kind::Scena => scena::decompile_json(o.title, &data).map(|_| ()),
			Kind::Table if dt::is_supported(&stem_of(&out_name)) => dt::decompile_xlsx(&data, &stem_of(&out_name), &out_name).map(|_| ()),
			_ => Ok(()),
		};
		if let Err(e) = back {
			return Outcome::Fail(lformat!("собранный файл не разбирается обратно — игра может его не прочитать: {e}", "the built file can't be parsed back, the game may not read it: {e}"));
		}
		Outcome::Ok(warn)
	});
	report(sink, tr("сборка", "build"), &summary);
	summary
}

fn top_dir(f: &Path, base: &Path) -> Option<String> {
	let rel = f.strip_prefix(base).ok()?;
	let mut c = rel.components();
	let first = c.next()?;
	c.next()?;
	Some(first.as_os_str().to_string_lossy().to_lowercase())
}

#[derive(Clone, Debug, Default)]
pub struct Ed7Folder {
	pub key: String,
	pub name: String,
	pub scenes: usize,
	pub tables: usize,
	pub textures: usize,
	pub others: usize,
	pub size: u64,
	pub unpacked: u64,
	pub out_size: u64,
	pub changed: usize,
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
	let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
	v.sort();
	v.retain(|p| {
		let n = file_name(p);
		!n.starts_with('.') && !n.starts_with("_backup")
	});
	v
}

pub fn scan_game_ed7(data: &Path) -> Vec<Ed7Folder> {
	let mut out = Vec::new();
	for d in subdirs(data) {
		let name = file_name(&d);
		let scena = name.to_lowercase().starts_with("scena");
		let mut files = Vec::new();
		walk(&d, &mut files);
		let mut f = Ed7Folder { key: name.to_lowercase(), name: name.clone(), ..Default::default() };
		for p in files {
			let l = file_name(&p).to_lowercase();
			let (n, k) = if l.ends_with(".bin") && scena {
				(&mut f.scenes, 4.3)
			} else if l.ends_with("._dt") || l.ends_with(".dt2") {
				(&mut f.tables, 1.3)
			} else if l.ends_with(".itp") {
				(&mut f.textures, 1.5)
			} else {
				(&mut f.others, 1.0)
			};
			*n += 1;
			let sz = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
			f.size += sz;
			f.unpacked += (sz as f64 * k) as u64;
		}
		out.push(f);
	}
	out
}

pub fn scan_project_ed7(proj: &Path) -> Vec<Ed7Folder> {
	let manifest = Manifest::load(proj);
	let mut out = Vec::new();
	for d in subdirs(proj) {
		let name = file_name(&d);
		let mut files = Vec::new();
		walk(&d, &mut files);
		let mut f = Ed7Folder { key: name.to_lowercase(), name: name.clone(), ..Default::default() };
		for p in files {
			let fname = file_name(&p);
			if fname.starts_with("~$") || fname.starts_with('.') {
				continue;
			}
			let l = fname.to_lowercase();
			let raw = p.strip_prefix(proj).ok().and_then(|r| manifest.map.get(&rel_key(r))).is_some_and(|a| a.1 == RAW);
			let (n, k) = if raw {
				(&mut f.others, 1.0)
			} else if l.ends_with(".json") {
				(&mut f.scenes, 4.3)
			} else if l.ends_with(".xlsx") {
				(&mut f.tables, 1.3)
			} else if l.ends_with(".png") {
				(&mut f.textures, 1.5)
			} else {
				continue;
			};
			*n += 1;
			let sz = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
			f.size += sz;
			let key = p.strip_prefix(proj).map(rel_key).unwrap_or_default();
			let same = match manifest.map.get(&key) {
				Some((h, _)) => std::fs::read(&p).map(|d| fnv(&d)).ok() == Some(*h),
				None => false,
			};
			if !same {
				f.changed += 1;
				f.out_size += (sz as f64 / k) as u64;
			}
		}
		out.push(f);
	}
	out
}

#[cfg(test)]
#[test]
fn mask_with_extension() {
	let m = |mask: &str, name: &str| mask_match(mask, name.split('.').next().unwrap()) || mask_match(mask, name);
	assert!(m("*.lay", "active_voice.lay") && !m("*.lay", "active_voice.dds") && !m("*.lay", "c0000.dat"));
	assert!(m("t_town._dt", "t_town._dt") && m("t_town._dt", "t_town.xlsx"));
	assert!(m("c01*", "c0100.dat") && !m("c01*", "c0200.dat"));
	assert!(m("face_c*,btlface0_c*", "btlface0_c0000.dds") && !m("face_c*", "noteface_c0000.dds"));
	assert!(m("", "anything.bin"));
}
