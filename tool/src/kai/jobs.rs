#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::{dds, p3a, scena, tbl, tbl_json};
use crate::jobs::{mask_match, Sink};

pub const MANIFEST: &str = ".kai_manifest.tsv";
pub const PATHS: &str = ".kai_paths.tsv";
pub const STATE: &str = ".kai_state.tsv";
pub const ARCHIVES: &str = ".kai_archives.tsv";
pub const MODFILES: &str = ".kai_modfiles.tsv";
pub const ORIG_DIR: &str = "_original";
pub const EXTRACT_DIR: &str = "_extracted";
pub const LIST_FILE: &str = "game_file_list.txt";
pub const ENGLISH_DIR: &str = "_english";
const ENGLISH_STATE: &str = ".kai_english.tsv";

#[derive(Clone, Debug)]
pub struct Options {
	pub game_dir: PathBuf,
	pub sources: Vec<PathBuf>,
	pub rebuild: bool,
	pub proj_dir: PathBuf,
	pub out_dir: PathBuf,
	pub mod_name: String,
	pub use_mods: bool,
	pub scena: bool,
	pub tables: bool,
	pub textures: bool,
	pub tex_dds: bool,
	pub tables_json: bool,
	pub check: bool,
	pub other: bool,
	pub mask: String,
	pub only_changed: bool,
	pub overwrite_edited: bool,
	pub game: Option<super::game::Game>,
	pub include: Option<Vec<String>>,
	pub build_only: Option<Vec<String>>,
}

pub fn pick_game(o: &Options) -> super::game::Game {
	use super::game;
	let g = o
		.game
		.or_else(|| if o.sources.is_empty() { game::detect(&o.game_dir) } else { o.sources.first().and_then(|f| game::detect(f.parent()?)) })
		.or_else(|| game::detect(&o.proj_dir))
		.unwrap_or(game::Game::Horizon);
	game::set(g);
	g
}

fn project_mismatch(o: &Options, g: super::game::Game) -> Option<String> {
	use super::game::Game;
	let pg = match std::fs::read_to_string(o.proj_dir.join(".kai_game.txt")).ok().and_then(|s| Game::from_key(&s)) {
		Some(pg) => pg,
		None => {
			let orig = o.proj_dir.join(ORIG_DIR);
			if orig.join("script_eng").is_dir() || orig.join("table_eng").is_dir() {
				Game::Horizon
			} else if (orig.join("script_en").is_dir() || orig.join("table_en").is_dir()) && g == Game::Horizon {
				return Some(lformat!("Папка проекта {} — для Trails through Daybreak, а сейчас выбрана {}. Укажите другую папку проекта.", "The project folder {} is for Trails through Daybreak, but {} is selected now. Choose another project folder.", o.proj_dir.display(), g.name()));
			} else {
				return None;
			}
		}
	};
	(pg != g).then(|| lformat!("Папка проекта {} — для игры {}, а сейчас выбрана {}. Укажите для этой игры другую папку проекта.", "The project folder {} is for {}, but {} is selected now. Choose another project folder for this game.", o.proj_dir.display(), pg.name(), g.name()))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
	Scena,
	Table,
	Yaml,
	Texture,
	Other,
}

impl Kind {
	fn code(self) -> &'static str {
		match self { Kind::Scena => "s", Kind::Table => "t", Kind::Yaml => "y", Kind::Texture => "x", Kind::Other => "o" }
	}
	fn from_code(c: &str) -> Option<Kind> {
		Some(match c { "s" => Kind::Scena, "t" => Kind::Table, "y" => Kind::Yaml, "x" => Kind::Texture, "o" => Kind::Other, _ => return None })
	}
	fn label(self) -> &'static str {
		match self {
			Kind::Scena => tr("скрипты (диалоги)", "scripts (dialogues)"),
			Kind::Table => tr("таблицы", "tables"),
			Kind::Yaml => tr("интерфейс (yaml)", "interface (yaml)"),
			Kind::Texture => tr("текстуры", "textures"),
			Kind::Other => tr("прочие файлы (как есть)", "other files (as they are)"),
		}
	}
}

#[derive(Default, Debug)]
pub struct Summary {
	pub ok: usize,
	pub skipped: usize,
	pub failed: usize,
	pub warnings: usize,
	pub same: usize,
	pub check: Vec<String>,
}

fn kind_of(vpath: &str) -> Option<Kind> {
	let p = vpath.to_lowercase();
	if (p.starts_with("script_eng/") || p.starts_with("script_en/") || p.starts_with("script/")) && p.ends_with(".dat") {
		Some(Kind::Scena)
	} else if (p.starts_with("table_eng/") || p.starts_with("table_en/") || p.starts_with("table/")) && p.ends_with(".tbl") {
		Some(Kind::Table)
	} else if p.starts_with("multilanguage/") && p.ends_with(".yaml") && !p.ends_with("_bk.yaml") {
		Some(Kind::Yaml)
	} else if p.starts_with("asset") && p.ends_with(".dds") && p.split('/').any(|s| s == "image" || s.starts_with("image_")) {
		Some(Kind::Texture)
	} else {
		None
	}
}

fn has_text(d: &[u8]) -> bool {
	for chunk in d.split(|&b| b == 0) {
		if chunk.len() < 6 {
			continue;
		}
		if let Ok(s) = std::str::from_utf8(chunk) {
			let letters = s.chars().filter(|c| c.is_alphabetic()).count();
			if s.contains(' ') && letters >= 5 && letters * 2 >= s.chars().count() {
				return true;
			}
			let jp = s.chars().filter(|&c| ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c)).count();
			if jp >= 3 && jp * 2 >= s.chars().count() {
				return true;
			}
		}
	}
	false
}

fn enabled(o: &Options, k: Kind) -> bool {
	match k {
		Kind::Scena => o.scena,
		Kind::Table | Kind::Yaml => o.tables,
		Kind::Texture => o.textures,
		Kind::Other => o.other,
	}
}

fn stem(vpath: &str) -> String {
	let name = vpath.rsplit('/').next().unwrap_or(vpath);
	name.split('.').next().unwrap_or(name).to_string()
}

fn fnv(d: &[u8]) -> u64 {
	let mut h: u64 = 0xcbf29ce484222325;
	for &b in d {
		h ^= b as u64;
		h = h.wrapping_mul(0x100000001b3);
	}
	h
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
	let Ok(rd) = std::fs::read_dir(dir) else { return };
	let mut v: Vec<_> = rd.flatten().map(|e| e.path()).collect();
	v.sort();
	for p in v {
		let n = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
		if p.is_dir() {
			if n == ORIG_DIR || n == EXTRACT_DIR || n == ENGLISH_DIR || n.starts_with("_backup") || n.starts_with('.') { continue }
			walk(&p, out);
		} else {
			out.push(p);
		}
	}
}

fn rel_str(p: &Path, base: &Path) -> String {
	p.strip_prefix(base).unwrap_or(p).components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

#[derive(Clone)]
enum Src {
	Loose(PathBuf),
	Packed(Arc<p3a::Archive>, usize),
}

impl Src {
	fn read(&self) -> Result<Vec<u8>, String> {
		match self {
			Src::Loose(p) => std::fs::read(p).map_err(|e| format!("{}: {e}", p.display())),
			Src::Packed(a, i) => a.read(&a.entries[*i]),
		}
	}
	fn folder(&self) -> &str {
		match self {
			Src::Loose(_) => "",
			Src::Packed(a, _) => &a.label,
		}
	}
	fn sig(&self) -> String {
		match self {
			Src::Loose(p) => stamp(p).map(|(s, t)| format!("{s}-{t}")).unwrap_or_default(),
			Src::Packed(a, i) => format!("{:016x}-{}", a.entries[*i].unc_hash, a.entries[*i].unc_size),
		}
	}
	fn size(&self) -> u64 {
		match self {
			Src::Loose(p) => std::fs::metadata(p).map(|m| m.len()).unwrap_or(0),
			Src::Packed(a, i) => a.entries[*i].unc_size,
		}
	}
}

fn source_files(o: &Options) -> Vec<(PathBuf, String)> {
	if o.sources.is_empty() {
		let mut files = Vec::new();
		walk(&o.game_dir, &mut files);
		return files
			.into_iter()
			.map(|f| { let r = rel_str(&f, &o.game_dir); (f, r) })
			.filter(|(_, r)| match &o.include {
				Some(inc) => {
					let l = r.to_lowercase();
					inc.iter().any(|i| l == *i || l.starts_with(&format!("{i}/")))
				}
				None => true,
			})
			.collect();
	}
	o.sources
		.iter()
		.map(|f| {
			let parent = f.parent().map(|p| p.to_path_buf()).unwrap_or_default();
			let in_mods = parent.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("mods")).unwrap_or(false);
			let base = if in_mods { parent.parent().map(|p| p.to_path_buf()).unwrap_or(parent.clone()) } else { parent };
			(f.clone(), rel_str(f, &base))
		})
		.collect()
}

pub fn source_base(o: &Options) -> PathBuf {
	if o.sources.is_empty() {
		return o.game_dir.clone();
	}
	let f = &o.sources[0];
	let parent = f.parent().map(|p| p.to_path_buf()).unwrap_or_default();
	if parent.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("mods")).unwrap_or(false) {
		parent.parent().map(|p| p.to_path_buf()).unwrap_or(parent)
	} else {
		parent
	}
}

fn index_game(o: &Options, sink: &dyn Sink) -> (BTreeMap<String, Src>, BTreeMap<String, Src>) {
	let (a, b, _) = index_game3(o, sink);
	(a, b)
}

fn index_game3(o: &Options, sink: &dyn Sink) -> (BTreeMap<String, Src>, BTreeMap<String, Src>, BTreeMap<String, Src>) {
	let files = source_files(o);
	let mut base: BTreeMap<String, Src> = BTreeMap::new();
	let mut mods: BTreeMap<String, Src> = BTreeMap::new();
	let mut loose: BTreeMap<String, Src> = BTreeMap::new();
	let mut extra: BTreeMap<String, Src> = BTreeMap::new();
	let mut archives = 0;
	for (f, rel) in files {
		let lrel = rel.to_lowercase();
		let in_mods = lrel.split('/').any(|c| c == "mods");
		if lrel.ends_with(".p3a") {
			if in_mods && !o.use_mods { continue }
			match p3a::Archive::open(&f) {
				Ok(mut a) => {
					archives += 1;
					a.label = rel[..rel.len() - 4].to_string();
					let a = Arc::new(a);
					let target = if in_mods { &mut mods } else { &mut base };
					for (i, e) in a.entries.iter().enumerate() {
						let v = e.name.to_lowercase();
						if in_mods {
							extra.insert(v.clone(), Src::Packed(a.clone(), i));
						}
						target.insert(v, Src::Packed(a.clone(), i));
					}
				}
				Err(e) => sink.log(&format!("⚠ {e}")),
			}
		} else if kind_of(&lrel).is_some() {
			loose.insert(lrel, Src::Loose(f));
		} else if let Some(pos) = lrel.find("/multilanguage/") {
			let v = lrel[pos + 1..].to_string();
			if kind_of(&v).is_some() { loose.insert(v, Src::Loose(f)); }
		}
	}
	sink.log(&lformat!("  архивов .p3a: {archives}{}", "  .p3a archives: {archives}{}", if o.use_mods { tr(" (включая папку mods)", " (including the mods folder)") } else { "" }));
	let shadow: BTreeMap<String, Src> = mods
		.keys()
		.filter(|v| matches!(kind_of(v), Some(Kind::Scena) | Some(Kind::Table)))
		.filter_map(|v| base.get(v).map(|s| (v.clone(), s.clone())))
		.collect();
	base.extend(mods);
	base.extend(loose);
	(base, extra, shadow)
}

#[derive(Default)]
struct Manifest {
	map: BTreeMap<String, u64>,
}

impl Manifest {
	fn load(dir: &Path) -> Manifest {
		let mut m = Manifest::default();
		if let Ok(s) = std::fs::read_to_string(dir.join(MANIFEST)) {
			for l in s.lines() {
				if let Some((p, h)) = l.split_once('\t') {
					if let Ok(h) = u64::from_str_radix(h.trim(), 16) {
						m.map.insert(p.to_string(), h);
					}
				}
			}
		}
		m
	}
	fn save(&self, dir: &Path) {
		let s: String = self.map.iter().map(|(p, h)| format!("{p}\t{h:016x}\n")).collect();
		let _ = write_atomic(&dir.join(MANIFEST), s.as_bytes());
	}
}

fn load_kv(p: &Path) -> BTreeMap<String, String> {
	let mut m = BTreeMap::new();
	if let Ok(s) = std::fs::read_to_string(p) {
		for l in s.lines() {
			let (k, v) = l.split_once('\t').unwrap_or((l, ""));
			if !k.is_empty() {
				m.insert(k.to_string(), v.to_string());
			}
		}
	}
	m
}

fn save_kv(p: &Path, m: &BTreeMap<String, String>) {
	let s: String = m.iter().map(|(k, v)| format!("{k}\t{v}\n")).collect();
	let _ = write_atomic(p, s.as_bytes());
}

fn stamp(p: &Path) -> Option<(u64, u64)> {
	let m = std::fs::metadata(p).ok()?;
	let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
	Some((m.len(), t.as_secs() * 1_000_000_000 + t.subsec_nanos() as u64))
}

fn load_state(dir: &Path) -> HashMap<String, (String, u64, u64)> {
	let mut m = HashMap::new();
	if let Ok(s) = std::fs::read_to_string(dir.join(STATE)) {
		for l in s.lines() {
			let v: Vec<&str> = l.split('\t').collect();
			if v.len() == 4 {
				if let (Ok(a), Ok(b)) = (v[2].parse(), v[3].parse()) {
					m.insert(v[0].to_string(), (v[1].to_string(), a, b));
				}
			}
		}
	}
	m
}

fn save_state(dir: &Path, m: &HashMap<String, (String, u64, u64)>) {
	let mut keys: Vec<&String> = m.keys().collect();
	keys.sort();
	let s: String = keys.iter().map(|k| { let (a, b, c) = &m[*k]; format!("{k}\t{a}\t{b}\t{c}\n") }).collect();
	let _ = write_atomic(&dir.join(STATE), s.as_bytes());
}

fn load_paths(dir: &Path) -> BTreeMap<String, (String, Kind)> {
	let mut m = BTreeMap::new();
	if let Ok(s) = std::fs::read_to_string(dir.join(PATHS)) {
		for l in s.lines() {
			let mut it = l.split('\t');
			if let (Some(p), Some(v), Some(k)) = (it.next(), it.next(), it.next()) {
				if let Some(k) = Kind::from_code(k) {
					m.insert(p.to_string(), (v.to_string(), k));
				}
			}
		}
	}
	m
}

fn save_paths(dir: &Path, m: &BTreeMap<String, (String, Kind)>) {
	let s: String = m.iter().map(|(p, (v, k))| format!("{p}\t{v}\t{}\n", k.code())).collect();
	let _ = write_atomic(&dir.join(PATHS), s.as_bytes());
}

enum Outcome {
	Ok(Option<String>),
	Skip(Option<String>),
	Same,
	Fail(String),
}

fn run_parallel<T: Sync>(tasks: &[T], name: impl Fn(&T) -> String + Sync, sink: &dyn Sink, f: impl Fn(&T) -> Outcome + Sync) -> Summary {
	let total = tasks.len();
	let next = AtomicUsize::new(0);
	let done = AtomicUsize::new(0);
	let summary = Mutex::new(Summary::default());
	let full = std::sync::atomic::AtomicBool::new(false);
	let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 8);
	sink.progress(0, total);
	std::thread::scope(|s| {
		for _ in 0..threads {
			s.spawn(|| loop {
				if sink.cancelled() || full.load(Ordering::SeqCst) { break }
				let i = next.fetch_add(1, Ordering::SeqCst);
				let Some(t) = tasks.get(i) else { break };
				sink.current(&name(t));
				let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(t))).unwrap_or_else(|_| Outcome::Fail(tr("внутренняя ошибка программы", "internal program error").into()));
				{
					let mut sm = summary.lock().unwrap();
					let n = name(t);
					match out {
						Outcome::Ok(None) => sm.ok += 1,
						Outcome::Ok(Some(w)) => { sm.ok += 1; sm.warnings += 1; sink.log(&format!("⚠ {n}: {w}")) }
						Outcome::Skip(Some(r)) => { sm.skipped += 1; sink.log(&format!("— {n}: {r}")) }
						Outcome::Skip(None) => sm.skipped += 1,
						Outcome::Same => sm.same += 1,
						Outcome::Fail(e) if disk_full(&e) => {
							sm.failed += 1;
							if !full.swap(true, Ordering::SeqCst) {
								sink.log(tr("✖ ДИСК ЗАПОЛНЕН — работа остановлена. Освободите место или выберите папку на другом диске и запустите снова", "✖ DISK FULL: work stopped. Free some space or choose a folder on another disk and run again"));
								sink.log(tr("  (уже готовые файлы останутся, повторный запуск продолжит с того же места).", "  (finished files stay, a new run continues from the same place)."));
							}
						}
						Outcome::Fail(e) => { sm.failed += 1; sink.log(&format!("✖ {n}: {e}")) }
					}
				}
				let d = done.fetch_add(1, Ordering::SeqCst) + 1;
				sink.progress(d, total);
			});
		}
	});
	summary.into_inner().unwrap()
}

fn report(sink: &dyn Sink, what: &str, s: &Summary) {
	sink.log(&lformat!("Готово ({what}): успешно {}{}, пропущено (нет текста или файл уже изменён вами) {}, ошибок {}{}", "Done ({what}): {} ok{}, {} skipped (no text or the file was edited by you), {} errors{}",
		s.ok,
		if s.same > 0 { lformat!(", уже были готовы {}", ", {} were already done", s.same) } else { String::new() },
		s.skipped,
		s.failed,
		if s.warnings > 0 { lformat!(", предупреждений {}", ", {} warnings", s.warnings) } else { String::new() }
	));
}

fn proj_name(vpath: &str, k: Kind) -> String {
	let base = vpath.rsplit_once('.').map(|x| x.0).unwrap_or(vpath);
	match k {
		Kind::Scena => format!("{base}.json"),
		Kind::Table => format!("{base}.xlsx"),
		Kind::Texture => format!("{base}.png"),
		Kind::Yaml | Kind::Other => vpath.to_string(),
	}
}

fn mask_ok(o: &Options, vpath: &str) -> bool {
	let name = vpath.rsplit('/').next().unwrap_or(vpath);
	mask_match(&o.mask, &stem(vpath)) || mask_match(&o.mask, name)
}

fn check_translation(tasks: &[(String, String, Kind, PathBuf)], originals: &BTreeMap<String, PathBuf>) -> Vec<String> {
	let tmp = std::env::temp_dir().join(format!("crossbell_check_{}", std::process::id()));
	let mut list = Vec::new();
	for (pname, vpath, k, f) in tasks.iter().filter(|t| matches!(t.2, Kind::Scena | Kind::Table)) {
		let Some(raw) = originals.get(vpath).and_then(|p| std::fs::read(p).ok()) else { continue };
		let en = match k {
			Kind::Scena => scena::decompile(&raw).ok().map(|(j, _)| j.into_bytes()),
			_ if pname.to_lowercase().ends_with(".json") => tbl_json::decompile(&raw).ok(),
			_ => tbl::decompile(&raw, vpath.rsplit('/').next().unwrap_or(vpath)).ok().map(|(b, _, _)| b),
		};
		let Some(en) = en else { continue };
		let p = tmp.join(pname);
		if let Some(d) = p.parent() {
			let _ = std::fs::create_dir_all(d);
		}
		if std::fs::write(&p, en).is_ok() {
			list.push((pname.clone(), p, f.clone()));
		}
	}
	let found = if list.is_empty() { Vec::new() } else { super::compare::check_files(&list) };
	let _ = std::fs::remove_dir_all(&tmp);
	found
}

pub fn log_check(sink: &dyn Sink, found: &[String]) {
	sink.log(&lformat!("✖ Сборка остановлена: в переводе сломаны служебные коды или подстановки — {}:", "✖ Build stopped: service codes or placeholders are broken in the translation: {}:", found.len()));
	for f in found.iter().take(30) {
		sink.log(&format!("  · {f}"));
	}
	if found.len() > 30 {
		sink.log(&lformat!("  … и ещё {}", "  … and {} more", found.len() - 30));
	}
	sink.log(tr("  Исправьте их (подробно — во вкладке «Проверки» Сверки) или соберите всё равно.", "  Fix them (details are in the Checks tab of CrossbellCompare) or build anyway."));
}

pub fn decompile(o: &Options, sink: &dyn Sink) -> Summary {
	if !(o.scena || o.tables || o.textures || o.other) {
		sink.log(tr("Не выбран ни один тип файлов.", "No file type selected."));
		return Summary::default();
	}
	if o.sources.is_empty() && !o.game_dir.is_dir() {
		sink.log(&lformat!("Папка игры не найдена: {}", "Game folder not found: {}", o.game_dir.display()));
		return Summary::default();
	}
	for f in &o.sources {
		if !f.is_file() {
			sink.log(&lformat!("Файл не найден: {}", "File not found: {}", f.display()));
			return Summary::default();
		}
	}
	let g = pick_game(o);
	if let Some(m) = project_mismatch(o, g) {
		sink.log(&format!("✖ {m}"));
		return Summary { failed: 1, ..Summary::default() };
	}
	sink.log(&lformat!("Декомпиляция ({}): {} → {}", "Decompiling ({}): {} → {}", g.name(), o.game_dir.display(), o.proj_dir.display()));
	if let Some(inc) = &o.include {
		sink.log(&lformat!("  выбрано: {}", "  selected: {}", inc.join(", ")));
	}
	let (index, extra, shadow) = index_game3(o, sink);
	let _ = std::fs::create_dir_all(&o.proj_dir);
	super::game::remember(&o.proj_dir, g);
	{
		let mut arch = load_kv(&o.proj_dir.join(ARCHIVES));
		for src in index.values().chain(extra.values()) {
			if let Src::Packed(a, _) = src {
				let abs = std::path::absolute(&a.path).unwrap_or(a.path.clone());
				arch.insert(a.label.clone(), abs.display().to_string());
			}
		}
		save_kv(&o.proj_dir.join(ARCHIVES), &arch);
		let mut mf = load_kv(&o.proj_dir.join(MODFILES));
		for v in extra.keys() {
			mf.insert(v.clone(), String::new());
		}
		save_kv(&o.proj_dir.join(MODFILES), &mf);
	}
	let mut n_extra = 0;
	if !extra.is_empty() {
		sink.log(tr("  проверяю файлы установленного мода…", "  checking the installed mod's files…"));
	}
	for (v, src) in &extra {
		let op = o.proj_dir.join(ORIG_DIR).join(v);
		let old = if op.metadata().map(|m| m.len()).ok() == Some(src.size()) { std::fs::read(&op).ok() } else { None };
		if let (Some(old), Src::Packed(a, i)) = (&old, src) {
			let h = a.entries[*i].unc_hash;
			if h != 0 && xxhash_rust::xxh64::xxh64(old, 0) == h {
				n_extra += 1;
				continue;
			}
		}
		if let Ok(d) = src.read() {
			if old.as_deref() == Some(d.as_slice()) {
				n_extra += 1;
				continue;
			}
			if let Some(p) = op.parent() { let _ = std::fs::create_dir_all(p); }
			if std::fs::write(&op, d).is_ok() { n_extra += 1 }
		}
	}
	if n_extra > 0 {
		sink.log(&lformat!("  файлов установленного мода: {n_extra} — все войдут в собранный архив (изменённые — в новом виде)", "  installed mod's files: {n_extra}, all go into the built archive (changed ones in their new form)"));
	}
	let mut tasks: Vec<(String, Kind, Src)> = Vec::new();
	for (v, src) in &index {
		let k = kind_of(v).unwrap_or(Kind::Other);
		if !enabled(o, k) || !mask_ok(o, v) { continue }
		tasks.push((v.clone(), k, src.clone()));
	}
	for k in [Kind::Scena, Kind::Table, Kind::Yaml, Kind::Texture, Kind::Other] {
		let n = tasks.iter().filter(|t| t.1 == k).count();
		if n > 0 { sink.log(&lformat!("  {}: {n} файлов", "  {}: {n} files", k.label())) }
	}
	let other_size: u64 = tasks.iter().filter(|t| t.1 == Kind::Other).map(|t| t.2.size()).sum();
	if other_size > 0 {
		sink.log(&lformat!("  объём прочих файлов: {:.1} МБ", "  size of other files: {:.1} MB", other_size as f64 / 1048576.0));
	}
	if tasks.is_empty() {
		sink.log(tr("Подходящих файлов не найдено. Укажите корневую папку игры (где лежат архивы .p3a и папка mods).", "No suitable files found. Choose the game's root folder (the one with the .p3a archives and the mods folder)."));
		return Summary::default();
	}
	let _ = std::fs::create_dir_all(&o.proj_dir);
	let manifest = Mutex::new(Manifest::load(&o.proj_dir));
	let paths = Mutex::new(load_paths(&o.proj_dir));
	let state = Mutex::new(load_state(&o.proj_dir));
	if let Some(free) = free_space(&o.proj_dir) {
		let need: u64 = tasks.iter().map(|t| if t.1 == Kind::Texture { t.2.size() * 3 } else { t.2.size() }).sum();
		if need > free {
			sink.log(&lformat!("⚠ Может не хватить места: нужно примерно {:.1} ГБ, свободно {:.1} ГБ.", "⚠ There may not be enough space: about {:.1} GB needed, {:.1} GB free.",
				need as f64 / 1073741824.0,
				free as f64 / 1073741824.0
			));
		}
	}
	let s = run_parallel(&tasks, |t| t.0.clone(), sink, |(v, k0, src)| {
		let mut k = *k0;
		let folder_join = |name: String| if src.folder().is_empty() { name } else { format!("{}/{}", src.folder(), name) };
		if k == Kind::Texture && o.tex_dds {
			k = Kind::Other;
		}
		let mut pname = folder_join(proj_name(v, k));
		if k == Kind::Table && o.tables_json {
			pname = pname.strip_suffix(".xlsx").map_or(pname.clone(), |b| format!("{b}.json"));
		}
		let dst = o.proj_dir.join(&pname);
		let sig = src.sig();
		let check = |pname: &str| -> Option<Outcome> {
			let dst = o.proj_dir.join(pname);
			if let Some((s0, sz, t)) = state.lock().unwrap().get(pname) {
				if *s0 == sig && stamp(&dst) == Some((*sz, *t)) {
					return Some(Outcome::Same);
				}
			}
			if dst.exists() && !o.overwrite_edited {
				let known = manifest.lock().unwrap().map.get(pname).copied();
				let cur = std::fs::read(&dst).map(|d| fnv(&d)).ok();
				if known != cur {
					return Some(Outcome::Skip(Some(tr("вы изменили этот файл — не перезаписан, чтобы не потерять правки", "you edited this file: not overwritten, so your edits aren't lost").into())));
				}
			}
			None
		};
		if let Some(r) = check(&pname) {
			return r;
		}
		let data = match src.read() { Ok(d) => d, Err(e) => return Outcome::Fail(e) };
		if let Some(p) = dst.parent() { let _ = std::fs::create_dir_all(p); }
		let save_orig = || -> Result<(), String> {
			let op = o.proj_dir.join(ORIG_DIR).join(v);
			if let Some(p) = op.parent() { let _ = std::fs::create_dir_all(p); }
			std::fs::write(&op, &data).map_err(|e| e.to_string())
		};
		if k == Kind::Texture && !dds::supported(&data) {
			k = Kind::Other;
			pname = folder_join(v.clone());
			if let Some(r) = check(&pname) {
				return r;
			}
		}
		let dst = o.proj_dir.join(&pname);
		if let Some(p) = dst.parent() { let _ = std::fs::create_dir_all(p); }
		let first_kind = k;
		let mut json_warn = None;
		let (bytes, warn): (Vec<u8>, Option<String>) = match k {
			Kind::Scena => match scena::decompile(&data) {
				Ok((json, _)) => {
					if let Err(e) = save_orig() { return Outcome::Fail(e) }
					(json.replace('\n', "\r\n").into_bytes(), None)
				}
				Err(_) if !has_text(&data) => {
					if let Err(e) = save_orig() { return Outcome::Fail(e) }
					k = Kind::Other;
					(Vec::new(), None)
				}
				Err(e) => return Outcome::Fail(e),
			},
			Kind::Table if o.tables_json => match tbl_json::decompile(&data) {
				Ok(j) => {
					if let Err(e) = save_orig() { return Outcome::Fail(e) }
					(j, None)
				}
				Err(e) => {
					json_warn = Some(lformat!("таблица не разобралась в JSON, в проекте как есть: {e}", "couldn't decompile the table to JSON, it stays in the project as is: {e}"));
					k = Kind::Other;
					(Vec::new(), None)
				}
			},
			Kind::Table => match tbl::decompile(&data, &pname) {
				Ok((_, 0, _)) => {
					k = Kind::Other;
					(Vec::new(), None)
				}
				Ok((book, _, _missing)) => {
					if let Err(e) = save_orig() { return Outcome::Fail(e) }
					(book, None)
				}
				Err(e) => return Outcome::Fail(e),
			},
			Kind::Texture => match dds::decompile(&data) {
				Ok(d) => {
					if let Err(e) = save_orig() { return Outcome::Fail(e) }
					(d.png, None)
				}
				Err(e) => return Outcome::Fail(e),
			},
			Kind::Yaml | Kind::Other => (data.clone(), None),
		};
		let (bytes, dst) = if k == Kind::Other && k != first_kind {
			pname = folder_join(v.clone());
			if let Some(r) = check(&pname) {
				return r;
			}
			let dst = o.proj_dir.join(&pname);
			if let Some(p) = dst.parent() { let _ = std::fs::create_dir_all(p); }
			(data.clone(), dst)
		} else {
			(bytes, dst)
		};
		let warn = warn.or(json_warn);
		if let Err(e) = std::fs::write(&dst, &bytes) {
			let _ = std::fs::remove_file(&dst);
			return Outcome::Fail(lformat!("не удалось записать: {e}", "could not write: {e}"));
		}
		paths.lock().unwrap().insert(pname.clone(), (v.clone(), k));
		if let Some((sz, t)) = stamp(&dst) {
			state.lock().unwrap().insert(pname.clone(), (sig, sz, t));
		}
		manifest.lock().unwrap().map.insert(pname, fnv(&bytes));
		Outcome::Ok(warn)
	});
	manifest.into_inner().unwrap().save(&o.proj_dir);
	save_paths(&o.proj_dir, &paths.into_inner().unwrap());
	save_state(&o.proj_dir, &state.into_inner().unwrap());
	english_originals(o, &shadow, sink);
	report(sink, tr("декомпиляция", "decompile"), &s);
	s
}

fn english_originals(o: &Options, shadow: &BTreeMap<String, Src>, sink: &dyn Sink) {
	let list: Vec<(&String, &Src, Kind)> = shadow
		.iter()
		.filter_map(|(v, s)| {
			let k = kind_of(v)?;
			(enabled(o, k) && mask_ok(o, v)).then_some((v, s, k))
		})
		.collect();
	if list.is_empty() || sink.cancelled() {
		return;
	}
	let dir = o.proj_dir.join(ENGLISH_DIR);
	let state = Mutex::new(load_kv(&o.proj_dir.join(ENGLISH_STATE)));
	let next = AtomicUsize::new(0);
	let made = AtomicUsize::new(0);
	let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 8);
	std::thread::scope(|sc| {
		for _ in 0..threads {
			sc.spawn(|| loop {
				let i = next.fetch_add(1, Ordering::SeqCst);
				let Some(&(v, src, k)) = list.get(i) else { break };
				if sink.cancelled() {
					break;
				}
				let dst = dir.join(proj_name(v, k));
				let sig = src.sig();
				if dst.exists() && state.lock().unwrap().get(v) == Some(&sig) {
					continue;
				}
				let Ok(data) = src.read() else { continue };
				let bytes = match k {
					Kind::Scena => scena::decompile(&data).ok().map(|(j, _)| j.replace('\n', "\r\n").into_bytes()),
					Kind::Table => tbl::decompile(&data, &proj_name(v, k)).ok().filter(|x| x.1 > 0).map(|x| x.0),
					_ => None,
				};
				if let Some(b) = bytes {
					if let Some(p) = dst.parent() {
						let _ = std::fs::create_dir_all(p);
					}
					if std::fs::write(&dst, b).is_ok() {
						state.lock().unwrap().insert(v.clone(), sig);
						made.fetch_add(1, Ordering::SeqCst);
					}
				}
			});
		}
	});
	save_kv(&o.proj_dir.join(ENGLISH_STATE), &state.into_inner().unwrap());
	let n = made.into_inner();
	if n > 0 {
		sink.log(&lformat!("  английские оригиналы для Сверки (файлы, заменённые модом): {n} → {ENGLISH_DIR}", "  English originals for CrossbellCompare (files replaced by the mod): {n} → {ENGLISH_DIR}"));
	}
}

pub fn compile(o: &Options, sink: &dyn Sink) -> Summary {
	if !o.proj_dir.is_dir() {
		sink.log(&lformat!("Папка проекта не найдена: {}", "Project folder not found: {}", o.proj_dir.display()));
		return Summary::default();
	}
	let g = pick_game(o);
	if let Some(m) = project_mismatch(o, g) {
		sink.log(&format!("✖ {m}"));
		return Summary { failed: 1, ..Summary::default() };
	}
	sink.log(&lformat!("Игра: {}", "Game: {}", g.name()));
	let manifest = Manifest::load(&o.proj_dir);
	let paths = load_paths(&o.proj_dir);
	let state = load_state(&o.proj_dir);
	let untouched = |pname: &str, f: &Path| -> bool {
		if let Some((_, sz, t)) = state.get(pname) {
			if stamp(f) == Some((*sz, *t)) {
				return true;
			}
		}
		match manifest.map.get(pname) {
			Some(h) => std::fs::read(f).map(|d| fnv(&d)).ok() == Some(*h),
			None => false,
		}
	};
	let orig_dir = o.proj_dir.join(ORIG_DIR);
	let mut originals: BTreeMap<String, PathBuf> = BTreeMap::new();
	{
		let mut v = Vec::new();
		walk_all(&orig_dir, &mut v);
		for f in v {
			let rel = rel_str(&f, &orig_dir).to_lowercase();
			if kind_of(&rel) == Some(Kind::Yaml) { continue }
			originals.insert(rel, f);
		}
	}
	let mut files = Vec::new();
	walk(&o.proj_dir, &mut files);
	let mut tasks: Vec<(String, String, Kind, PathBuf)> = Vec::new();
	let mut unchanged = 0;
	for f in files {
		let pname = rel_str(&f, &o.proj_dir);
		let low = pname.to_lowercase();
		let fname = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
		if fname.starts_with("~$") || fname.starts_with('.') { continue }
		if let Some(only) = &o.build_only {
			if !only.iter().any(|p| low.starts_with(p.as_str())) { continue }
		}
		let base = pname.rsplit_once('.').map(|x| x.0.to_string()).unwrap_or(pname.clone());
		let (k, vpath) = if let Some((v, k)) = paths.get(&pname) {
			(*k, v.clone())
		} else if low.ends_with(".json") {
			let head = std::fs::read(&f).map(|d| String::from_utf8_lossy(&d[..d.len().min(200)]).into_owned()).unwrap_or_default();
			if !scena::looks_like(&head) { continue }
			(Kind::Scena, format!("{base}.dat"))
		} else if low.ends_with(".xlsx") {
			(Kind::Table, format!("{base}.tbl"))
		} else if low.ends_with(".png") {
			(Kind::Texture, format!("{base}.dds"))
		} else if kind_of(&low) == Some(Kind::Yaml) {
			(Kind::Yaml, pname.clone())
		} else {
			continue;
		};
		let vpath = vpath.to_lowercase();
		if k == Kind::Other {
			if untouched(&pname, &f) {
				continue;
			}
			tasks.push((pname, vpath, k, f));
			continue;
		}
		if !enabled(o, k) || !mask_ok(o, &vpath) { continue }
		if o.only_changed && (k == Kind::Yaml || originals.contains_key(&vpath)) {
			if untouched(&pname, &f) {
				unchanged += 1;
				continue;
			}
		}
		tasks.push((pname, vpath, k, f));
	}
	sink.log(&lformat!("Сборка: {} → {}", "Building: {} → {}", o.proj_dir.display(), o.out_dir.display()));
	if let Some(only) = &o.build_only {
		sink.log(&lformat!("  выбрано: {}", "  selected: {}", only.iter().map(|x| x.trim_end_matches('/')).collect::<Vec<_>>().join(", ")));
	}
	for k in [Kind::Scena, Kind::Table, Kind::Yaml, Kind::Texture, Kind::Other] {
		let n = tasks.iter().filter(|t| t.2 == k).count();
		if n > 0 {
			if k == Kind::Other {
				sink.log(&lformat!("  изменённые прочие файлы (войдут в архив как есть): {n}", "  changed other files (go into the archive as they are): {n}"))
			} else {
				sink.log(&lformat!("  пересобрать — {}: {n} файлов", "  rebuild {}: {n} files", k.label()))
			}
		}
	}
	{
		let mut seen: HashMap<&str, &str> = HashMap::new();
		for t in &tasks {
			if let Some(prev) = seen.insert(t.1.as_str(), t.0.as_str()) {
				sink.log(&lformat!("⚠ {} и {} — это один и тот же файл игры; в архив попадёт только один из них", "⚠ {} and {} are the same game file; only one of them will go into the archive", prev, t.0));
			}
		}
	}
	if unchanged > 0 {
		sink.log(&lformat!("  без изменений (берутся исходные файлы): {unchanged}", "  unchanged (source files are used): {unchanged}"));
	}
	if o.check {
		let found = check_translation(&tasks, &originals);
		if !found.is_empty() {
			log_check(sink, &found);
			return Summary { check: found, ..Summary::default() };
		}
	}
	if tasks.is_empty() && originals.is_empty() {
		sink.log(if unchanged > 0 || !load_paths(&o.proj_dir).is_empty() { tr("Нечего собирать — изменённых файлов нет.", "Nothing to build: no changed files.") } else { tr("Нечего собирать — сначала сделайте декомпиляцию в эту папку.", "Nothing to build: decompile into this folder first.") });
		return Summary::default();
	}
	if o.rebuild && !tasks.is_empty() {
		let _ = std::fs::create_dir_all(&o.out_dir);
		let arch = load_kv(&o.proj_dir.join(ARCHIVES));
		let size_of = |label: &String, path: &String| {
			let p = PathBuf::from(path);
			let p = if p.is_file() { p } else { o.game_dir.join(format!("{label}.p3a")) };
			std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
		};
		let mut used: HashSet<&String> = HashSet::new();
		let mut edited = 0u64;
		for (pname, ..) in &tasks {
			edited += std::fs::metadata(o.proj_dir.join(pname)).map(|m| m.len()).unwrap_or(0);
			let low = pname.to_lowercase();
			if let Some(l) = arch.keys().filter(|l| low.starts_with(&format!("{}/", l.to_lowercase()))).max_by_key(|l| l.len()) {
				used.insert(l);
			}
		}
		let archives: u64 = used.iter().map(|l| size_of(*l, &arch[*l])).sum();
		let need = rebuild_space(archives, edited);
		if let Some(free) = free_space(&o.out_dir).filter(|&f| f < need) {
			sink.log(&lformat!("✖ НЕ ХВАТИТ МЕСТА: для сборки нужно примерно {:.1} ГБ (временный файл и новые архивы), а на диске папки для собранного свободно {:.1} ГБ. Сборка не начиналась — освободите место или выберите другую папку.", "✖ NOT ENOUGH SPACE: the build needs about {:.1} GB (a temporary file and the new archives), but the disk of the output folder has {:.1} GB free. The build did not start. Free some space or choose another folder.",
				need as f64 / 1073741824.0,
				free as f64 / 1073741824.0
			));
			return Summary { failed: 1, ..Summary::default() };
		}
	}
	let results: Mutex<HashMap<String, Vec<u8>>> = Mutex::new(HashMap::new());
	let spool = if o.rebuild && !tasks.is_empty() {
		let _ = std::fs::create_dir_all(&o.out_dir);
		match p3a::Spool::create(&o.out_dir.join(".build_spool.tmp")) {
			Ok(sp) => Some(sp),
			Err(e) => {
				sink.log(&lformat!("✖ не удалось создать временный файл сборки: {e}", "✖ can't create the temporary build file: {e}"));
				return Summary { failed: 1, ..Summary::default() };
			}
		}
	} else {
		None
	};
	let spooled: Mutex<HashMap<String, p3a::Spooled>> = Mutex::new(HashMap::new());
	let yaml_out: Mutex<Vec<(String, Vec<u8>)>> = Mutex::new(Vec::new());
	let mut s = run_parallel(&tasks, |t| t.0.clone(), sink, |(_pname, vpath, k, f)| {
		let data = match std::fs::read(f) { Ok(d) => d, Err(e) => return Outcome::Fail(lformat!("не удалось прочитать (закройте файл в других программах): {e}", "could not read (close the file in other programs): {e}")) };
		let orig = || originals.get(vpath).and_then(|p| std::fs::read(p).ok()).ok_or_else(|| lformat!("нет исходного файла в {ORIG_DIR} — декомпилируйте этот файл заново", "no source file in {ORIG_DIR}: decompile this file again"));
		let (bytes, warn) = match k {
			Kind::Scena => {
				let src = String::from_utf8_lossy(&data).into_owned();
				match scena::compile(&src) { Ok(b) => (b, None), Err(e) => return Outcome::Fail(e) }
			}
			Kind::Table => {
				let or = match orig() { Ok(x) => x, Err(e) => return Outcome::Fail(e) };
				let built = if _pname.to_lowercase().ends_with(".json") { tbl_json::compile(&data, &or).map(|b| (b, Vec::new())) } else { tbl::compile_checked(&data, &or) };
				match built {
					Ok((b, sk)) if sk.is_empty() => (b, None),
					Ok((b, sk)) => (b, Some(lformat!("служебные поля (имена файлов/моделей) переводить нельзя — оставлены как в оригинале: {}", "service fields (file and model names) must not be translated, left as in the original: {}", sk.join("; ")))),
					Err(e) => return Outcome::Fail(e),
				}
			}
			Kind::Texture => {
				let or = match orig() { Ok(x) => x, Err(e) => return Outcome::Fail(e) };
				match dds::compile(&data, &or) { Ok(r) => r, Err(e) => return Outcome::Fail(e) }
			}
			Kind::Yaml => {
				yaml_out.lock().unwrap().push((vpath.clone(), data));
				return Outcome::Ok(None);
			}
			Kind::Other => (data, None),
		};
		if let Some(sp) = &spool {
			match sp.put(&bytes) {
				Ok(x) => {
					spooled.lock().unwrap().insert(vpath.clone(), x);
				}
				Err(e) => return Outcome::Fail(lformat!("не удалось записать временный файл сборки (хватает ли места на диске?): {e}", "can't write the temporary build file (is there enough disk space?): {e}")),
			}
		} else {
			results.lock().unwrap().insert(vpath.clone(), bytes);
		}
		Outcome::Ok(warn)
	});
	if sink.cancelled() {
		sink.log(tr("Остановлено — архив не записан.", "Stopped: the archive was not written."));
		return s;
	}
	if s.failed > 0 {
		sink.log(tr("⚠ В архив для файлов с ошибками взяты исходные версии.", "⚠ Source versions were used in the archive for files with errors."));
	}
	let mut built = results.into_inner().unwrap();
	if o.rebuild {
		let arch = load_kv(&o.proj_dir.join(ARCHIVES));
		let mut ready = spooled.into_inner().unwrap();
		let mut by_label: BTreeMap<String, HashMap<String, p3a::Spooled>> = BTreeMap::new();
		for (pname, vpath, k, _) in &tasks {
			let Some(b) = ready.remove(vpath) else { continue };
			let tail = proj_name(vpath, *k);
			let label = pname.strip_suffix(tail.as_str()).map(|x| x.trim_end_matches('/').to_string()).unwrap_or_default();
			let label = if label.is_empty() {
				arch.keys().filter(|l| pname.to_lowercase().starts_with(&format!("{}/", l.to_lowercase()))).max_by_key(|l| l.len()).cloned().unwrap_or_default()
			} else {
				label
			};
			if label.is_empty() {
				sink.log(&lformat!("⚠ {pname}: не понятно, к какому архиву относится файл — пропущен", "⚠ {pname}: unclear which archive the file belongs to, skipped"));
				continue;
			}
			by_label.entry(label).or_default().insert(vpath.clone(), b);
		}
		if by_label.is_empty() {
			sink.log(tr("Изменённых файлов для архивов нет — архивы не пересобирались.", "No changed files for the archives: archives were not rebuilt."));
		}
		let src_of = |label: &String| arch.get(label).map(PathBuf::from).filter(|p| p.is_file()).unwrap_or_else(|| o.game_dir.join(format!("{label}.p3a")));
		let need: u64 = by_label.keys().map(|l| std::fs::metadata(src_of(l)).map(|m| m.len()).unwrap_or(0)).sum();
		if let Some(free) = free_space(&o.out_dir).filter(|&f| f < need + (need >> 5)) {
			s.failed += 1;
			sink.log(&lformat!("✖ НЕ ХВАТИТ МЕСТА: архивы ({}) займут ≈ {:.1} ГБ, а на диске папки для собранного свободно {:.1} ГБ. Архивы не собирались — освободите место или выберите другую папку.", "✖ NOT ENOUGH SPACE: the archives ({}) need ≈ {:.1} GB, but the output folder's disk has {:.1} GB free. The archives were not built: free some space or choose another folder.",
				by_label.len(),
				need as f64 / 1073741824.0,
				free as f64 / 1073741824.0
			));
			by_label.clear();
		}
		for (label, replace) in &by_label {
			let src_path = src_of(label);
			let a = match p3a::Archive::open(&src_path) {
				Ok(a) => a,
				Err(e) => {
					sink.log(&lformat!("✖ {label}: нет исходного архива ({e}). Укажите папку игры или файл, из которого декомпилировали.", "✖ {label}: no source archive ({e}). Choose the game folder or the file you decompiled from."));
					continue;
				}
			};
			let dst = o.out_dir.join(format!("{label}.p3a"));
			if same_file(&dst, &src_path) {
				sink.log(&lformat!("✖ {}: нельзя собирать поверх исходного архива — выберите другую папку для собранного", "✖ {}: can't build over the source archive, choose another output folder", dst.display()));
				continue;
			}
			let _ = std::fs::create_dir_all(dst.parent().unwrap());
			sink.log(&lformat!("  собираю {label}.p3a …", "  building {label}.p3a …"));
			let Some(sp) = &spool else { continue };
			match p3a::rebuild(&a, replace, sp, &dst) {
				Ok((n, r)) => {
					sink.log(&lformat!("Архив: {} — файлов {n}, заменено {r}", "Archive: {}, {n} files, {r} replaced", dst.display()));
					match p3a::verify_spooled(&dst, replace.iter().map(|(k, v)| (k.as_str(), *v))) {
						Ok(c) => sink.log(&lformat!("  проверено: архив читается, заменённые файлы ({c}) на месте", "  verified: the archive reads, replaced files ({c}) are in place")),
						Err(e) => {
							s.failed += 1;
							sink.log(&lformat!("✖ {label}.p3a записан с ошибкой: {e}. Соберите заново; если повторится — проверьте диск", "✖ {label}.p3a was written with an error: {e}. Build again; if it repeats, check the disk"));
						}
					}
				}
				Err(e) => sink.log(&format!("✖ {label}.p3a: {e}")),
			}
		}
		for (v, d) in yaml_out.into_inner().unwrap() {
			let dst = o.out_dir.join(&v);
			let _ = std::fs::create_dir_all(dst.parent().unwrap());
			match write_atomic(&dst, &d) {
				Ok(()) => sink.log(&lformat!("Файл интерфейса: {}", "Interface file: {}", dst.display())),
				Err(e) => sink.log(&lformat!("✖ не удалось записать {}: {e}", "✖ could not write {}: {e}", dst.display())),
			}
		}
		sink.log(tr("Пересобраны только архивы, в которых что-то изменено. Скопируйте их в папку игры с заменой (оригиналы сохраните).", "Only archives with changes were rebuilt. Copy them into the game folder, replacing the files (keep the originals)."));
		report(sink, tr("сборка", "build"), &s);
		return s;
	}
	let modfiles = load_kv(&o.proj_dir.join(MODFILES));
	if !modfiles.is_empty() {
		originals.retain(|v, _| modfiles.contains_key(v));
	}
	let mut packed: Vec<(String, Vec<u8>)> = Vec::new();
	let mut from_orig = 0;
	for (v, p) in &originals {
		if let Some(b) = built.remove(v) {
			packed.push((v.clone(), b));
		} else if let Ok(d) = std::fs::read(p) {
			from_orig += 1;
			packed.push((v.clone(), d));
		}
	}
	packed.extend(built);
	packed.sort_by(|a, b| a.0.cmp(&b.0));
	if !packed.is_empty() {
		let name = mod_file_name(&o.mod_name);
		let dst = o.out_dir.join("mods").join(format!("{name}.p3a"));
		let _ = std::fs::create_dir_all(dst.parent().unwrap());
		let data = p3a::write(&packed);
		let written = match free_space(&dst).filter(|&f| f < data.len() as u64 + (1 << 20)) {
			Some(free) => {
				s.failed += 1;
				sink.log(&lformat!("✖ НЕ ХВАТИТ МЕСТА: архив мода — {:.1} МБ, а на диске свободно {:.1} МБ. Архив не записан — освободите место или выберите другую папку.", "✖ NOT ENOUGH SPACE: the mod archive is {:.1} MB, but the disk has {:.1} MB free. The archive was not written: free some space or choose another folder.", data.len() as f64 / 1048576.0, free as f64 / 1048576.0));
				None
			}
			None => Some(write_atomic(&dst, &data)),
		};
		match written {
			None => {}
			Some(Ok(())) => {
				sink.log(&lformat!("Архив мода: {} ({} файлов, из них без изменений {from_orig}; {:.1} МБ)", "Mod archive: {} ({} files, {from_orig} of them unchanged; {:.1} MB)",
					dst.display(),
					packed.len(),
					data.len() as f64 / 1048576.0
				));
				drop(data);
				match p3a::verify(&dst, packed.iter().map(|(k, v)| (k.as_str(), v.as_slice()))) {
					Ok(c) => sink.log(&lformat!("  проверено: архив читается, все {c} файлов совпадают с собранными", "  verified: the archive reads, all {c} files match the built ones")),
					Err(e) => {
						s.failed += 1;
						sink.log(&lformat!("✖ архив мода записан с ошибкой: {e}. Соберите заново; если повторится — проверьте диск", "✖ the mod archive was written with an error: {e}. Build again; if it repeats, check the disk"));
					}
				}
			}
			Some(Err(e)) => sink.log(&lformat!("✖ не удалось записать {}: {e}", "✖ could not write {}: {e}", dst.display())),
		}
		let n_scena = packed.iter().filter(|x| kind_of(&x.0) == Some(Kind::Scena)).count();
		if n_scena < 1000 && packed.len() < 1000 {
			sink.log(tr("ℹ В архив попадают только файлы, декомпилированные в эту папку проекта. Если этот архив должен заменить", "ℹ Only files decompiled into this project folder go into the archive. If this archive must replace"));
			sink.log(tr("  весь мод (например RU_MOD.p3a), один раз декомпилируйте все типы файлов без маски — дальше можно работать по маске.", "  a whole mod (for example RU_MOD.p3a), decompile all file types once without a mask; after that you can work with a mask."));
		}
	}
	for (v, d) in yaml_out.into_inner().unwrap() {
		let dst = o.out_dir.join(&v);
		let _ = std::fs::create_dir_all(dst.parent().unwrap());
		match write_atomic(&dst, &d) {
			Ok(()) => sink.log(&lformat!("Файл интерфейса: {}", "Interface file: {}", dst.display())),
			Err(e) => sink.log(&lformat!("✖ не удалось записать {}: {e}", "✖ could not write {}: {e}", dst.display())),
		}
	}
	report(sink, tr("сборка", "build"), &s);
	s
}

fn same_file(a: &Path, b: &Path) -> bool {
	if a == b {
		return true;
	}
	match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
		(Ok(x), Ok(y)) => x == y,
		_ => {
			let norm = |p: &Path| std::path::absolute(p).unwrap_or(p.to_path_buf()).to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
			norm(a) == norm(b)
		}
	}
}

pub fn mod_file_name(name: &str) -> String {
	let n: String = name.trim().trim_end_matches(".p3a").chars().map(|c| if "\\/:*?\"<>|".contains(c) || c.is_control() { '_' } else { c }).collect();
	let n = n.trim().trim_end_matches('.').to_string();
	if n.is_empty() { "KAI_MOD".to_string() } else { n }
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

fn walk_all(dir: &Path, out: &mut Vec<PathBuf>) {
	let Ok(rd) = std::fs::read_dir(dir) else { return };
	for e in rd.flatten() {
		let p = e.path();
		if p.is_dir() { walk_all(&p, out) } else { out.push(p) }
	}
}

pub fn extract(o: &Options, sink: &dyn Sink) -> Summary {
	if !o.game_dir.is_dir() {
		sink.log(&lformat!("Папка игры не найдена: {}", "Game folder not found: {}", o.game_dir.display()));
		return Summary::default();
	}
	let _ = std::fs::create_dir_all(&o.proj_dir);
	let mut files = Vec::new();
	walk(&o.game_dir, &mut files);
	let mut list = String::from(tr("\u{FEFF}архив\tфайл\tразмер (байт)\r\n", "\u{FEFF}archive\tfile\tsize (bytes)\r\n"));
	let mut index: BTreeMap<String, Src> = BTreeMap::new();
	let mut n_arch = 0;
	for f in files {
		let rel = rel_str(&f, &o.game_dir);
		let lrel = rel.to_lowercase();
		if !lrel.ends_with(".p3a") { continue }
		if !o.sources.is_empty() && !o.sources.iter().any(|s| same_file(s, &f)) { continue }
		let in_mods = lrel.split('/').any(|c| c == "mods");
		if in_mods && !o.use_mods && o.sources.is_empty() { continue }
		match p3a::Archive::open(&f) {
			Ok(mut a) => {
				n_arch += 1;
				a.label = rel[..rel.len() - 4].to_string();
				let a = Arc::new(a);
				for (i, e) in a.entries.iter().enumerate() {
					list.push_str(&format!("{rel}\t{}\t{}\r\n", e.name, e.unc_size));
					let v = e.name.to_lowercase();
					if in_mods || !index.contains_key(&v) {
						index.insert(v, Src::Packed(a.clone(), i));
					}
				}
			}
			Err(e) => sink.log(&format!("⚠ {e}")),
		}
	}
	let lp = o.proj_dir.join(LIST_FILE);
	let _ = std::fs::write(&lp, list);
	sink.log(&lformat!("Архивов: {n_arch}, файлов в них: {}. Полный список: {}", "Archives: {n_arch}, files in them: {}. Full list: {}", index.len(), lp.display()));
	if o.mask.trim().is_empty() {
		sink.log(tr("Чтобы извлечь файлы, укажите маску, например: chr0000*  (или имя файла из списка).", "To extract files, give a mask, for example: chr0000*  (or a file name from the list)."));
		return Summary::default();
	}
	let tasks: Vec<(String, Src)> = index
		.iter()
		.filter(|(v, _)| {
			let name = v.rsplit('/').next().unwrap_or(v);
			mask_match(&o.mask, &stem(v)) || mask_match(&o.mask, name)
		})
		.map(|(v, s)| (v.clone(), s.clone()))
		.collect();
	let total: u64 = tasks.iter().map(|(_, s)| match s { Src::Packed(a, i) => a.entries[*i].unc_size, _ => 0 }).sum();
	sink.log(&lformat!("Подходит под маску: {} файлов, {:.1} МБ", "Matches the mask: {} files, {:.1} MB", tasks.len(), total as f64 / 1048576.0));
	if tasks.is_empty() {
		return Summary::default();
	}
	let root = o.proj_dir.join(EXTRACT_DIR);
	let s = run_parallel(&tasks, |t| t.0.clone(), sink, |(v, src)| {
		let data = match src.read() { Ok(d) => d, Err(e) => return Outcome::Fail(e) };
		let dst = root.join(src.folder()).join(v);
		if let Some(p) = dst.parent() { let _ = std::fs::create_dir_all(p); }
		match std::fs::write(&dst, data) {
			Ok(()) => Outcome::Ok(None),
			Err(e) => Outcome::Fail(e.to_string()),
		}
	});
	sink.log(&lformat!("Файлы извлечены в {}", "Files extracted to {}", root.display()));
	report(sink, tr("извлечение", "extraction"), &s);
	s
}

pub fn plan(o: &Options) -> (usize, u64) {
	struct Quiet;
	impl Sink for Quiet {
		fn log(&self, _: &str) {}
		fn progress(&self, _: usize, _: usize) {}
	}
	let (index, _) = index_game(o, &Quiet);
	let mut n = 0;
	let mut size = 0;
	for (v, src) in &index {
		let k = kind_of(v).unwrap_or(Kind::Other);
		if enabled(o, k) && mask_ok(o, v) {
			n += 1;
			size += if k == Kind::Texture { src.size() * 3 } else { src.size() };
		}
	}
	(n, size)
}

pub fn rebuild_space(archives: u64, edited: u64) -> u64 {
	let need = archives + edited.min(archives);
	need + (need >> 5)
}

pub fn free_space(path: &Path) -> Option<u64> {
	let mut p = path.to_path_buf();
	while !p.exists() {
		p = p.parent()?.to_path_buf();
	}
	#[cfg(windows)]
	{
		use std::os::windows::ffi::OsStrExt;
		let w: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
		let mut free: u64 = 0;
		let ok = unsafe { windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(w.as_ptr(), &mut free, std::ptr::null_mut(), std::ptr::null_mut()) };
		if ok != 0 { Some(free) } else { None }
	}
	#[cfg(not(windows))]
	{
		let out = std::process::Command::new("df").arg("-k").arg(&p).output().ok()?;
		let s = String::from_utf8_lossy(&out.stdout);
		s.lines().nth(1)?.split_whitespace().nth(3)?.parse::<u64>().ok().map(|k| k * 1024)
	}
}

fn disk_full(msg: &str) -> bool {
	msg.contains("os error 112") || msg.contains("os error 28") || msg.to_lowercase().contains("no space left")
}

#[derive(Clone, Default, Debug)]
pub struct Counts {
	pub scenes: usize,
	pub scripts: usize,
	pub tables: usize,
	pub images: usize,
	pub fonts: usize,
	pub yaml: usize,
	pub other: usize,
}

impl Counts {
	fn add(&mut self, name: &str) {
		let n = name.to_lowercase().replace('\\', "/");
		let scena = n.contains("/scena/") || n.starts_with("scena/");
		if n.ends_with(".dat") || n.ends_with(".json") {
			if scena { self.scenes += 1 } else { self.scripts += 1 }
		} else if n.ends_with(".tbl") || n.ends_with(".xlsx") {
			self.tables += 1
		} else if n.ends_with(".dds") || n.ends_with(".png") {
			self.images += 1
		} else if n.ends_with(".fnt") {
			self.fonts += 1
		} else if n.ends_with(".yaml") {
			self.yaml += 1
		} else {
			self.other += 1
		}
	}
	pub fn total(&self) -> usize {
		self.scenes + self.scripts + self.tables + self.images + self.fonts + self.yaml + self.other
	}
}

#[derive(Clone, Debug)]
pub struct ScanItem {
	pub key: String,
	pub name: String,
	pub size: u64,
	pub unpacked: u64,
	pub counts: Counts,
	pub in_mods: bool,
	pub files: Vec<String>,
	pub error: Option<String>,
}

fn scan_p3a(p: &Path, key: String, name: String, in_mods: bool) -> ScanItem {
	let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
	let mut counts = Counts::default();
	let mut error = None;
	let mut unpacked = 0;
	match p3a::Archive::open(p) {
		Ok(a) => {
			for e in &a.entries {
				counts.add(&e.name);
				unpacked += if e.name.to_lowercase().ends_with(".dds") { e.unc_size * 3 } else { e.unc_size };
			}
		}
		Err(e) => error = Some(e),
	}
	ScanItem { key, name, size, unpacked, counts, in_mods, files: Vec::new(), error }
}

pub fn scan_game(dir: &Path) -> Vec<ScanItem> {
	let mut out = Vec::new();
	let Ok(rd) = std::fs::read_dir(dir) else { return out };
	let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
	v.sort();
	for p in &v {
		let n = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
		let low = n.to_lowercase();
		if p.is_file() && low.ends_with(".p3a") {
			out.push(scan_p3a(p, low.clone(), n.clone(), false));
		} else if p.is_dir() && low == "mods" {
			let mut m: Vec<PathBuf> = std::fs::read_dir(p).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default();
			m.sort();
			for f in m {
				let fname = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
				if f.is_file() && fname.to_lowercase().ends_with(".p3a") {
					out.push(scan_p3a(&f, format!("{low}/{}", fname.to_lowercase()), format!("{n}\\{fname}"), true));
				}
			}
		} else if p.is_dir() && low == "multilanguage" {
			let mut files = Vec::new();
			let mut size = 0;
			let mut counts = Counts::default();
			let mut all = Vec::new();
			walk(p, &mut all);
			for f in all {
				let rel = format!("multilanguage/{}", rel_str(&f, p));
				if kind_of(&rel) == Some(Kind::Yaml) {
					size += std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
					counts.add(&rel);
					files.push(f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
				}
			}
			if !files.is_empty() {
				out.push(ScanItem { key: low.clone(), name: format!("{n}\\"), size, unpacked: size, counts, in_mods: false, files, error: None });
			}
		}
	}
	out
}

#[derive(Clone, Debug)]
pub struct ProjItem {
	pub key: String,
	pub name: String,
	pub kind: String,
	pub in_mods: bool,
	pub counts: Counts,
	pub changed: usize,
	pub archive: String,
	pub archive_size: u64,
	pub mod_size: u64,
}

pub fn scan_project(proj: &Path) -> Vec<ProjItem> {
	let arch = load_kv(&proj.join(ARCHIVES));
	let labels: Vec<String> = arch.keys().cloned().collect();
	let mod_size: u64 = {
		let mut v = Vec::new();
		walk_all(&proj.join(ORIG_DIR), &mut v);
		v.iter().map(|f| std::fs::metadata(f).map(|m| m.len()).unwrap_or(0)).sum()
	};
	let manifest = Manifest::load(proj);
	let state = load_state(proj);
	let mut files = Vec::new();
	walk(proj, &mut files);
	let mut map: BTreeMap<String, ProjItem> = BTreeMap::new();
	for f in files {
		let pname = rel_str(&f, proj);
		let fname = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
		if fname.starts_with("~$") || fname.starts_with('.') || !pname.contains('/') { continue }
		let low = pname.to_lowercase();
		let label = labels.iter().filter(|l| low.starts_with(&format!("{}/", l.to_lowercase()))).max_by_key(|l| l.len());
		let (prefix_len, kind) = match label {
			Some(l) => {
				let rest = &pname[l.len() + 1..];
				match rest.split_once('/') {
					Some((first, _)) => (l.len() + 1 + first.len(), first.to_string()),
					None => (l.len(), l.rsplit('/').next().unwrap_or(l).to_string()),
				}
			}
			None => {
				let first = pname.split('/').next().unwrap_or("").to_string();
				(first.len(), first)
			}
		};
		let disp = &pname[..prefix_len];
		let key = format!("{}/", disp.to_lowercase());
		let changed = match state.get(&pname) {
			Some((_, sz, t)) if stamp(&f) == Some((*sz, *t)) => false,
			_ => match manifest.map.get(&pname) {
				Some(h) => std::fs::read(&f).map(|d| fnv(&d)).ok() != Some(*h),
				None => true,
			},
		};
		let e = map.entry(key.clone()).or_insert_with(|| ProjItem {
			key: key.clone(),
			name: format!("{}\\", disp.replace('/', "\\")),
			kind: kind.to_lowercase(),
			in_mods: low.starts_with("mods/"),
			counts: Counts::default(),
			changed: 0,
			archive: label.cloned().unwrap_or_default(),
			archive_size: label.and_then(|l| arch.get(l)).and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len()).unwrap_or(0),
			mod_size,
		});
		e.counts.add(&pname);
		if changed {
			e.changed += 1;
		}
	}
	map.into_values().collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn mod_names() {
		assert_eq!(mod_file_name("RU/MOD"), "RU_MOD");
		assert_eq!(mod_file_name(" RU_MOD.p3a "), "RU_MOD");
		assert_eq!(mod_file_name("..."), "KAI_MOD");
	}
}
