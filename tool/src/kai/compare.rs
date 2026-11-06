#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

static ED7: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn set_ed7(t: Option<crate::scena::Title>) {
	let v = match t {
		None => 0,
		Some(crate::scena::Title::Zero) => 1,
		Some(crate::scena::Title::Azure) => 2,
	};
	ED7.store(v, std::sync::atomic::Ordering::Relaxed);
}

pub fn ed7() -> Option<crate::scena::Title> {
	match ED7.load(std::sync::atomic::Ordering::Relaxed) {
		1 => Some(crate::scena::Title::Zero),
		2 => Some(crate::scena::Title::Azure),
		_ => None,
	}
}

#[derive(Clone, Debug, Default)]
pub struct Side {
	pub text: String,
	pub path: PathBuf,
	pub line: usize,
}

#[derive(Clone, Debug)]
pub struct Pair {
	pub file: String,
	pub id: String,
	pub en: Option<Side>,
	pub ru: Option<Side>,
	pub en_key: String,
	pub ru_key: String,
	pub service: bool,
	pub who: Option<u32>,
}

#[derive(Default)]
pub struct Data {
	pub pairs: Vec<Pair>,
	pub files: BTreeMap<String, (usize, usize)>,
	pub warnings: Vec<String>,
	pub game_warning: Option<String>,
}

pub fn clean(s: &str, keep_newlines: bool) -> String {
	let mut out = String::with_capacity(s.len());
	let mut chars = s.chars().peekable();
	while let Some(c) = chars.next() {
		match c {
			'<' => {
				let rest: String = chars.clone().take(41).collect();
				if let Some(p) = rest.find('>') {
					for _ in 0..rest[..p].chars().count() + 1 {
						chars.next();
					}
				} else {
					out.push(c);
				}
			}
			'{' => {
				let rest: String = chars.clone().take(20).collect();
				if let Some(p) = rest.find('}') {
					let inner = &rest[..p];
					let ok = |x: char| x.is_ascii_alphanumeric() || x == '-' || (ed7().is_some() && (x == ' ' || x == '[' || x == ']' || x == '_'));
					if inner.chars().all(ok) && (!inner.is_empty() || ed7().is_some()) {
						for _ in 0..inner.chars().count() + 1 {
							chars.next();
						}
						continue;
					}
				}
				out.push(c);
			}
			'#' if ed7().is_some() && chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) => {
				let rest: String = chars.clone().take(14).collect();
				let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
				if rest.chars().nth(digits).map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
					for _ in 0..digits + 1 {
						chars.next();
					}
				} else {
					out.push(c);
				}
			}
			'\n' => out.push(if keep_newlines { '\n' } else { ' ' }),
			'\r' => {}
			'㈱' if ed7().is_some() => out.push('♥'),
			'㍉' if ed7().is_some() => out.push('è'),
			_ => out.push(c),
		}
	}
	out.trim().to_string()
}

pub fn search_key(s: &str) -> String {
	fold(&clean(s, false).to_lowercase()).replace('…', "...")
}

fn fold(s: &str) -> String {
	s.chars()
		.filter(|&c| c != '\u{301}' && c != '\u{300}')
		.map(|c| match c {
			'ё' => 'е',
			'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
			'è' | 'é' | 'ê' | 'ë' => 'e',
			'ì' | 'í' | 'î' | 'ï' => 'i',
			'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
			'ù' | 'ú' | 'û' | 'ü' => 'u',
			'ç' => 'c',
			'ñ' => 'n',
			'ÿ' => 'y',
			'─' | '—' | '–' | '‒' | '―' | '‐' | '‑' | '−' => '-',
			'«' | '»' | '“' | '”' | '„' | '‟' | '\'' | '‘' | '’' => '"',
			'\u{a0}' | '\u{2009}' | '\u{202f}' => ' ',
			c => c,
		})
		.collect()
}

struct Item {
	key: String,
	id: String,
	text: String,
	line: usize,
}

fn strip_ln(code: &str) -> String {
	let b = code.as_bytes();
	let mut out = String::with_capacity(code.len());
	let mut i = 0;
	while i < b.len() {
		if b[i].is_ascii_digit() && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_')) {
			let mut j = i;
			while j < b.len() && b[j].is_ascii_digit() {
				j += 1;
			}
			if j < b.len() && b[j] == b'@' {
				i = j + 1;
				continue;
			}
			out.push_str(&code[i..j]);
			i = j;
			continue;
		}
		let ch = code[i..].chars().next().unwrap();
		out.push(ch);
		i += ch.len_utf8();
	}
	out
}

fn plain_strings(code: &str) -> String {
	let mut out = String::with_capacity(code.len());
	let mut it = code.char_indices().peekable();
	while let Some((i, c)) = it.next() {
		if c != '"' {
			out.push(c);
			continue;
		}
		let mut end = None;
		let mut esc = false;
		for (j, d) in it.by_ref() {
			if esc {
				esc = false;
			} else if d == '\\' {
				esc = true;
			} else if d == '"' {
				end = Some(j);
				break;
			}
		}
		match end {
			Some(j) if !code[i + 1..j].is_ascii() => out.push_str("\"…\""),
			Some(j) => out.push_str(&code[i..=j]),
			None => out.push_str(&code[i..]),
		}
	}
	out
}

fn speaker_of(key: &str) -> Option<u32> {
	key.split_once('|')?.1.strip_prefix("system[5,6](")?.split(',').next()?.trim().parse().ok()
}

pub fn speaker_face(game: &str, id: u32) -> Option<u32> {
	static T: std::sync::OnceLock<Vec<(&'static str, u32, u32)>> = std::sync::OnceLock::new();
	let t = T.get_or_init(|| {
		include_str!("speakers.tsv")
			.lines()
			.filter_map(|l| {
				let mut c = l.split('\t');
				Some((c.next()?, c.next()?.parse().ok()?, c.next()?.parse().ok()?))
			})
			.collect()
	});
	t.iter().find(|x| x.0 == game && x.1 == id).map(|x| x.2)
}

fn read_json(path: &Path) -> Result<Vec<Item>, String> {
	let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
	let v: serde_json::Value = serde_json::from_str(src.trim_start_matches('\u{feff}')).map_err(|e| lformat!("ошибка JSON: {e}", "JSON error: {e}"))?;
	if v.get("kiseki").and_then(|k| k.as_str()) == Some(crate::scena_json::MAGIC) {
		return Ok(read_ed7_json(&src, &v, true));
	}
	if v.get("headers").is_some() && v.get("kiseki").is_none() {
		return Ok(super::tbl_json::texts(&v).into_iter().enumerate().map(|(n, (id, text))| Item { key: id.clone(), id, text, line: n + 2 }).collect());
	}
	if v.get("kiseki").and_then(|k| k.as_str()) != Some("kai-scena") {
		return Err(tr("не файл диалогов CrossbellTool", "not a dialogue file of CrossbellTool").into());
	}
	let lines_of_texts = text_lines(&src);
	let mut out = Vec::new();
	let mut k = 0;
	let mut n_line = 0;
	let braces = |l: &str| -> i32 {
		let mut d = 0;
		let mut q = false;
		let mut esc = false;
		for c in l.chars() {
			if esc { esc = false; continue }
			match c {
				'\\' if q => esc = true,
				'"' => q = !q,
				'{' if !q => d += 1,
				'}' if !q => d -= 1,
				_ => {}
			}
		}
		d
	};
	let mut depth = 0i32;
	let mut calls: Option<(i32, usize, Option<usize>)> = None;
	let mut func = String::new();
	for it in v.get("script").and_then(|s| s.as_array()).map(|a| a.as_slice()).unwrap_or(&[]) {
		if let Some(line) = it.as_str() {
			let t = line.trim();
			let before = depth;
			depth += braces(t);
			if before == 0 && t.starts_with("fn ") {
				func = t[3..].split(['(', ' ']).next().unwrap_or("").to_string();
			}
			if calls.is_none() && t.starts_with("fn ") && t.ends_with(" calls {") {
				calls = Some((before, out.len(), None));
			} else if let Some((d0, b1, b2)) = calls {
				if b2.is_none() && t == "} {" && depth == d0 + 1 {
					calls = Some((d0, b1, Some(out.len())));
				} else if depth == d0 {
					if let Some(b2) = b2 {
						let (n1, n2) = (b2 - b1, out.len() - b2);
						let cyr = |r: &[Item]| r.iter().any(|it| it.text.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)));
						if n1 > 0 && n2 > 0 && (n1 % n2 == 0 || (!cyr(&out[b1..b2]) && cyr(&out[b2..]))) {
							out.drain(b1..b2);
						}
					}
					calls = None;
				}
			}
			continue;
		}
		let Some(obj) = it.as_object() else { continue };
		let code = obj.get("code").and_then(|c| c.as_str()).unwrap_or("");
		let key = plain_strings(&strip_ln(code).replace(", \"\");", ");"));
		n_line += 1;
		for (i, t) in obj.get("text").and_then(|t| t.as_array()).map(|a| a.as_slice()).unwrap_or(&[]).iter().enumerate() {
			let text = t.as_str().unwrap_or("").to_string();
			let line = lines_of_texts.get(k).copied().unwrap_or(0);
			k += 1;
			out.push(Item { key: format!("{func}|{key}#{i}"), id: lformat!("реплика {n_line}", "line {n_line}"), text, line });
		}
	}
	Ok(out)
}

fn text_lines(src: &str) -> Vec<usize> {
	let mut out = Vec::new();
	let mut in_text = false;
	for (n, l) in src.lines().enumerate() {
		let t = l.trim();
		if in_text {
			if t.starts_with(']') {
				in_text = false;
			} else if t.starts_with('"') {
				out.push(n + 1);
			}
		} else if let Some(rest) = t.strip_prefix("\"text\": [") {
			let rest = rest.trim();
			if rest.is_empty() {
				in_text = true;
			} else if rest.starts_with('"') {
				let n_items = serde_json::from_str::<Vec<serde_json::Value>>(&format!("[{}", rest.trim_end_matches(','))).map(|v| v.len()).unwrap_or(1);
				out.extend(std::iter::repeat(n + 1).take(n_items));
			}
		}
	}
	out
}

fn read_ed7_json(src: &str, v: &serde_json::Value, with_lines: bool) -> Vec<Item> {
	let lines = if with_lines { text_lines(src) } else { Vec::new() };
	let mut out = Vec::new();
	let mut k = 0usize;
	let mut n = 0usize;
	fn go(a: &[serde_json::Value], out: &mut Vec<Item>, k: &mut usize, n: &mut usize, lines: &[usize]) {
		for it in a {
			let Some(o) = it.as_object() else { continue };
			let line = o.get("line").and_then(|l| l.as_str()).unwrap_or("");
			if let Some(t) = o.get("text").and_then(|t| t.as_array()) {
				*n += 1;
				for (i, x) in t.iter().enumerate() {
					let ln = lines.get(*k).copied().unwrap_or(0);
					*k += 1;
					out.push(Item { key: format!("{line}#{i}"), id: lformat!("реплика {}", "line {}", *n), text: x.as_str().unwrap_or("").to_string(), line: ln });
				}
			}
			if let Some(b) = o.get("body").and_then(|b| b.as_array()) {
				go(b, out, k, n, lines);
			}
		}
	}
	go(v.get("script").and_then(|s| s.as_array()).map(|a| a.as_slice()).unwrap_or(&[]), &mut out, &mut k, &mut n, &lines);
	out
}

fn read_bin(path: &Path) -> Result<Vec<Item>, String> {
	let data = std::fs::read(path).map_err(|e| e.to_string())?;
	let first = ed7().unwrap_or(crate::scena::Title::Zero);
	let json = crate::scena::quick_json(first, &data).or_else(|e| crate::scena::quick_json(first.other(), &data).map_err(|_| e))?;
	let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
	Ok(read_ed7_json(&json, &v, false))
}

fn read_dt(path: &Path) -> Result<Vec<Item>, String> {
	let data = std::fs::read(path).map_err(|e| e.to_string())?;
	let n = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
	let stem = n.split('.').next().unwrap_or(&n).to_string();
	Ok(crate::dt::texts(&data, &stem)?.into_iter().map(|(id, key, text)| Item { key, id, text, line: 0 }).collect())
}

fn read_xlsx(path: &Path) -> Result<Vec<Item>, String> {
	if ed7().is_some() {
		let book = std::fs::read(path).map_err(|e| e.to_string())?;
		return Ok(crate::dt::texts_xlsx(&book)?.into_iter().enumerate().map(|(n, (id, key, text))| Item { key, id, text: text.replace("\r\n", "\n"), line: n + 2 }).collect());
	}
	use calamine::{Data, Reader, Xlsx};
	let mut wb: Xlsx<_> = calamine::open_workbook(path).map_err(|e| lformat!("не удалось открыть Excel: {e}", "could not open Excel: {e}"))?;
	let ws = wb.worksheet_range(super::tbl::SHEET).or_else(|_| wb.worksheet_range(super::tbl::LEGACY_SHEET)).map_err(|_| tr("нет листа «Text»", "no 'Text' sheet").to_string())?;
	let row0 = ws.start().map(|(r, _)| r as usize).unwrap_or(0);
	let cell = |d: Option<&Data>| match d {
		Some(Data::String(s)) => s.clone(),
		Some(Data::Empty) | None => String::new(),
		Some(o) => o.to_string(),
	};
	let mut out = Vec::new();
	for (r, row) in ws.rows().enumerate().skip(1) {
		let id = cell(row.first());
		if id.trim().is_empty() {
			continue;
		}
		out.push(Item { key: id.clone(), id, text: cell(row.get(1)).replace("\r\n", "\n"), line: row0 + r + 1 });
	}
	Ok(out)
}

fn unquote_yaml(v: &str) -> String {
	let v = v.trim();
	if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
		let inner = &v[1..v.len() - 1];
		let mut out = String::new();
		let mut it = inner.chars();
		while let Some(c) = it.next() {
			if c == '\\' {
				match it.next() {
					Some('n') => out.push('\n'),
					Some('t') => out.push('\t'),
					Some(x) => out.push(x),
					None => {}
				}
			} else {
				out.push(c);
			}
		}
		out
	} else if v.len() >= 2 && v.starts_with('\'') && v.ends_with('\'') {
		v[1..v.len() - 1].replace("''", "'")
	} else {
		v.to_string()
	}
}

fn read_yaml(path: &Path) -> Result<Vec<Item>, String> {
	let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
	let mut out = Vec::new();
	for (n, l) in src.lines().enumerate() {
		let t = l.trim_start_matches('\u{feff}');
		if t.starts_with('#') || t.trim().is_empty() || t.starts_with(' ') {
			continue;
		}
		if let Some((k, v)) = t.split_once(':') {
			out.push(Item { key: k.trim().to_string(), id: k.trim().to_string(), text: unquote_yaml(v), line: n + 1 });
		}
	}
	Ok(out)
}

fn cyr_ratio(path: &Path) -> f64 {
	let s = std::fs::read_to_string(path).unwrap_or_default();
	let letters = s.chars().filter(|c| c.is_alphabetic()).count().max(1);
	s.chars().filter(|&c| ('А'..='я').contains(&c) || c == 'ё' || c == 'Ё').count() as f64 / letters as f64
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
	let Ok(rd) = std::fs::read_dir(dir) else { return };
	for e in rd.flatten() {
		let n = e.file_name().to_string_lossy().to_string();
		let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
		if is_dir {
			let l = n.to_lowercase();
			let ed7_skip = ed7().is_some() && (l.starts_with("map") || l.starts_with("visual") || l.starts_with("etc") || l.starts_with("movie") || l.starts_with("voice") || l.starts_with("bgm") || l.starts_with("chr") || l.starts_with("face") || l.starts_with("effect") || l.starts_with("battle") || l.starts_with("monster") || l == "test" || l.starts_with("system") || l.starts_with("savefile") || l.ends_with("shader") || l == "se");
			if ed7_skip || n == "_original" || n == "_extracted" || n.starts_with('.') || n.starts_with("_backup")
				|| ["asset_shader", "shader", "vfx", "model", "model_info", "image", "scene", "layout_pc", "layout_pc_eng", "system_pc"].contains(&l.as_str())
			{
				continue;
			}
			walk(&e.path(), out);
		} else {
			let l = n.to_lowercase();
			let ed7_raw = ed7().is_some() && (l.ends_with(".bin") || l.ends_with("._dt") || l.ends_with(".dt2"));
			if !n.starts_with("~$") && (l.ends_with(".json") || l.ends_with(".xlsx") || l.ends_with(".yaml") || ed7_raw) {
				out.push(e.path());
			}
		}
	}
}

fn file_key(rel: &str) -> Option<String> {
	let l = rel.to_lowercase().replace('\\', "/");
	if ed7().is_some() {
		let name = l.rsplit('/').next()?;
		let stem = name.split('.').next().unwrap_or(name);
		let dirs: Vec<&str> = l.split('/').collect();
		let dirs = &dirs[..dirs.len() - 1];
		if (name.ends_with(".json") || name.ends_with(".bin")) && dirs.iter().any(|d| d.starts_with("scena")) {
			return Some(format!("scena/{stem}"));
		}
		if (name.ends_with(".xlsx") || name.ends_with("._dt") || name.ends_with(".dt2")) && !stem.contains(' ') && crate::dt::is_supported(stem) {
			return Some(format!("table/{}", stem.strip_suffix("_us").filter(|s| *s == "monsnote").unwrap_or(stem)));
		}
		return None;
	}
	if l.ends_with(".json") && l.split('/').any(|d| d == "table" || d == "table_en" || d == "table_eng") {
		let name = l.rsplit('/').next()?;
		return Some(format!("table/{}.xlsx", name.trim_end_matches(".json")));
	}
	if l.ends_with(".json") {
		let p = l.rfind("script_eng/").map(|p| p + "script_eng/".len())
			.or_else(|| l.rfind("script_en/").map(|p| p + "script_en/".len()))
			.or_else(|| l.rfind("script/").map(|p| p + "script/".len()))?;
		Some(l[p..].to_string())
	} else if l.ends_with(".xlsx") {
		let name = l.rsplit('/').next()?;
		Some(format!("table/{name}"))
	} else if l.ends_with(".yaml") && l.rsplit('/').next()?.starts_with("message_") {
		Some("multilanguage/message.yaml".into())
	} else {
		None
	}
}

fn english_from_game(dir: &Path) -> Option<PathBuf> {
	let mut arcs: Vec<PathBuf> = std::fs::read_dir(dir)
		.ok()?
		.flatten()
		.map(|e| e.path())
		.filter(|p| {
			let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
			n.starts_with("script") && (n.ends_with("_en.p3a") || n.ends_with("_eng.p3a"))
		})
		.collect();
	if arcs.is_empty() {
		return None;
	}
	arcs.sort();
	if let Some(g) = super::game::detect(dir) {
		super::game::set(g);
	}
	let sig: String = std::iter::once("v2;".to_string())
		.chain(arcs
		.iter()
		.map(|p| {
			let m = std::fs::metadata(p).ok();
			format!("{}|{}|{:?};", p.display(), m.as_ref().map(|m| m.len()).unwrap_or(0), m.and_then(|m| m.modified().ok()))
		}))
		.collect();
	let out = std::env::current_exe().ok()?.parent()?.join("CrossbellCompare_cache").join(format!("en_{:016x}", fnv64(sig.as_bytes())));
	if out.join(".done").is_file() {
		return Some(out);
	}
	let _ = std::fs::remove_dir_all(&out);
	for a in &arcs {
		let Ok(arc) = super::p3a::Archive::open(a) else { continue };
		let stem = a.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
		let next = std::sync::atomic::AtomicUsize::new(0);
		let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 8);
		std::thread::scope(|s| {
			for _ in 0..threads {
				s.spawn(|| loop {
					let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
					let Some(e) = arc.entries.get(i) else { break };
					let n = e.name.to_lowercase().replace('\\', "/");
					let name = n.rsplit('/').next().unwrap_or(&n).to_string();
					let conv: Option<(String, Vec<u8>)> = if n.ends_with(".dat") {
						arc.read(e).ok().and_then(|d| super::scena::decompile(&d).ok()).map(|(j, _)| (format!("{}.json", n.trim_end_matches(".dat")), j.replace('\n', "\r\n").into_bytes()))
					} else if n.ends_with(".tbl") {
						arc.read(e).ok().and_then(|d| super::tbl::decompile(&d, &name).ok()).filter(|x| x.1 > 0).map(|x| (format!("{}.xlsx", n.trim_end_matches(".tbl")), x.0))
					} else {
						None
					};
					if let Some((rel, data)) = conv {
						let dst = out.join(&stem).join(rel);
						if let Some(p) = dst.parent() {
							let _ = std::fs::create_dir_all(p);
						}
						let _ = std::fs::write(dst, data);
					}
				});
			}
		});
	}
	let _ = std::fs::create_dir_all(&out);
	let _ = std::fs::write(out.join(".done"), b"");
	Some(out)
}

fn collect(dir: &Path, russian: bool) -> HashMap<String, Vec<PathBuf>> {
	let mut files = Vec::new();
	walk(dir, &mut files);
	let from_game = if !russian && ed7().is_none() { english_from_game(dir) } else { None };
	if let Some(c) = &from_game {
		walk(c, &mut files);
	}
	let mut m: HashMap<String, Vec<PathBuf>> = HashMap::new();
	for f in files {
		let base = from_game.as_deref().filter(|c| f.starts_with(c)).unwrap_or(dir);
		let rel = f.strip_prefix(base).unwrap_or(&f).to_string_lossy().replace('\\', "/");
		if let Some(k) = file_key(&rel) {
			m.entry(k).or_default().push(f);
		}
	}
	for v in m.values_mut() {
		if russian {
			v.retain(|p| {
				let l = p.to_string_lossy().replace('\\', "/").to_lowercase();
				!l.contains("/_english/") && !l.contains("/data_backup/")
			});
		}
		if ed7().is_some() {
			v.sort_by_key(|p| {
				let l = p.to_string_lossy().replace('\\', "/").to_lowercase();
				let backup = l.contains("/data_backup/");
				let us = l.contains("_us/") || l.contains("monsnote_us.");
				let raw = l.ends_with(".bin") || l.ends_with("._dt") || l.ends_with(".dt2");
				((if russian { backup } else { !backup }) as u8, !us as u8, raw as u8)
			});
			continue;
		}
		v.sort_by_key(|p| {
			let l = p.to_string_lossy().replace('\\', "/").to_lowercase();
			let in_mods = l.contains("/mods/");
			let eng = l.contains("/_english/");
			if russian { (!in_mods) as u8 } else if eng { 0 } else if in_mods { 2 } else { 1 }
		});
	}
	m.retain(|_, v| !v.is_empty());
	m
}

fn align_by_func(a: &[Item], b: &[Item]) -> Vec<(Option<usize>, Option<usize>)> {
	fn blocks(v: &[Item]) -> Vec<(&str, usize, usize)> {
		let mut out: Vec<(&str, usize, usize)> = Vec::new();
		for (i, it) in v.iter().enumerate() {
			let f = it.key.split('|').next().unwrap_or("");
			match out.last_mut() {
				Some(l) if l.0 == f => l.2 = i + 1,
				_ => out.push((f, i, i + 1)),
			}
		}
		out
	}
	let (ba, bb) = (blocks(a), blocks(b));
	let (x, y) = (ba.len(), bb.len());
	let w = y + 1;
	let mut dp = vec![0u32; (x + 1) * w];
	for i in (0..x).rev() {
		for j in (0..y).rev() {
			dp[i * w + j] = if ba[i].0 == bb[j].0 { dp[(i + 1) * w + j + 1] + 1 } else { dp[(i + 1) * w + j].max(dp[i * w + j + 1]) };
		}
	}
	let mut out = Vec::new();
	let (mut i, mut j) = (0, 0);
	let one = |out: &mut Vec<(Option<usize>, Option<usize>)>, a0: usize, a1: usize, b0: usize, b1: usize| {
		if (a1 - a0) * (b1 - b0) > 30_000_000 {
			for k in 0..(a1 - a0).max(b1 - b0) {
				out.push(((a0 + k < a1).then_some(a0 + k), (b0 + k < b1).then_some(b0 + k)));
			}
		} else {
			out.extend(align(&a[a0..a1], &b[b0..b1]).into_iter().map(|(p, q)| (p.map(|p| a0 + p), q.map(|q| b0 + q))));
		}
	};
	while i < x || j < y {
		if i < x && j < y && ba[i].0 == bb[j].0 {
			one(&mut out, ba[i].1, ba[i].2, bb[j].1, bb[j].2);
			i += 1;
			j += 1;
		} else if j >= y || (i < x && dp[(i + 1) * w + j] >= dp[i * w + j + 1]) {
			out.extend((ba[i].1..ba[i].2).map(|k| (Some(k), None)));
			i += 1;
		} else {
			out.extend((bb[j].1..bb[j].2).map(|k| (None, Some(k))));
			j += 1;
		}
	}
	out
}

fn lead_tag(s: &str) -> &str {
	if s.starts_with('<') { s.split_inclusive('>').next().unwrap_or("") } else { "" }
}

fn align(a: &[Item], b: &[Item]) -> Vec<(Option<usize>, Option<usize>)> {
	let n = a.len();
	let m = b.len();
	let same = |i: usize, j: usize| a[i].key == b[j].key && lead_tag(&a[i].text) == lead_tag(&b[j].text);
	let mut pre = 0;
	while pre < n && pre < m && same(pre, pre) {
		pre += 1;
	}
	let mut suf = 0;
	while suf < n - pre && suf < m - pre && same(n - 1 - suf, m - 1 - suf) {
		suf += 1;
	}
	let (a1, b1) = (&a[pre..n - suf], &b[pre..m - suf]);
	let mut mid: Vec<(Option<usize>, Option<usize>)> = Vec::new();
	let (x, y) = (a1.len(), b1.len());
	if x * y > 30_000_000 {
		for (i, j) in align_by_func(a1, b1) {
			mid.push((i.map(|i| pre + i), j.map(|j| pre + j)));
		}
	} else if x > 0 || y > 0 {
		let w = y + 1;
		let (la, lb): (Vec<&str>, Vec<&str>) = (a1.iter().map(|t| lead_tag(&t.text)).collect(), b1.iter().map(|t| lead_tag(&t.text)).collect());
		let score = |i: usize, j: usize| -> u16 { if a1[i].key != b1[j].key { 0 } else if la[i] == lb[j] { 3 } else { 2 } };
		let mut dp = vec![0u16; (x + 1) * w];
		for i in (0..x).rev() {
			for j in (0..y).rev() {
				let s = score(i, j);
				let skip = dp[(i + 1) * w + j].max(dp[i * w + j + 1]);
				dp[i * w + j] = if s > 0 { skip.max(dp[(i + 1) * w + j + 1] + s) } else { skip };
			}
		}
		let (mut i, mut j) = (0, 0);
		let mut pend_a = Vec::new();
		let mut pend_b = Vec::new();
		let flush = |pa: &mut Vec<usize>, pb: &mut Vec<usize>, mid: &mut Vec<(Option<usize>, Option<usize>)>| {
			let k = pa.len().max(pb.len());
			for t in 0..k {
				mid.push((pa.get(t).map(|v| pre + v), pb.get(t).map(|v| pre + v)));
			}
			pa.clear();
			pb.clear();
		};
		while i < x || j < y {
			if i < x && j < y && score(i, j) > 0 && dp[i * w + j] == dp[(i + 1) * w + j + 1] + score(i, j) {
				flush(&mut pend_a, &mut pend_b, &mut mid);
				mid.push((Some(pre + i), Some(pre + j)));
				i += 1;
				j += 1;
			} else if j >= y || (i < x && dp[(i + 1) * w + j] >= dp[i * w + j + 1]) {
				pend_a.push(i);
				i += 1;
			} else {
				pend_b.push(j);
				j += 1;
			}
		}
		flush(&mut pend_a, &mut pend_b, &mut mid);
	}
	let mut out: Vec<(Option<usize>, Option<usize>)> = (0..pre).map(|i| (Some(i), Some(i))).collect();
	out.extend(mid);
	out.extend((0..suf).map(|k| (Some(n - suf + k), Some(m - suf + k))));
	out
}

fn read_any(path: &Path) -> Result<Vec<Item>, String> {
	let l = path.to_string_lossy().to_lowercase();
	if l.ends_with(".json") {
		read_json(path)
	} else if l.ends_with(".bin") {
		read_bin(path)
	} else if l.ends_with("._dt") || l.ends_with(".dt2") {
		read_dt(path)
	} else if l.ends_with(".xlsx") {
		read_xlsx(path)
	} else {
		read_yaml(path)
	}
}

pub fn load(en_dir: &Path, ru_dir: &Path, progress: &(dyn Fn(usize, usize) + Sync)) -> Data {
	load_cached(en_dir, ru_dir, progress, None)
}

pub fn load_cached(en_dir: &Path, ru_dir: &Path, progress: &(dyn Fn(usize, usize) + Sync), cache: Option<&Path>) -> Data {
	let cache_file = cache.map(|d| d.join(format!("{:016x}.bin", fnv64(format!("{}|{}|{}", en_dir.display(), ru_dir.display(), ED7.load(std::sync::atomic::Ordering::Relaxed)).as_bytes()))));
	let old: HashMap<String, cache::Entry> = cache_file.as_deref().and_then(cache::read).unwrap_or_default();
	if ed7().is_none() {
		if let Some(g) = super::game::detect(en_dir).or_else(|| super::game::detect(ru_dir)) {
			super::game::set(g);
		}
	}
	let en = collect(en_dir, false);
	let ru = collect(ru_dir, true);
	let mut keys: Vec<String> = en.keys().chain(ru.keys()).cloned().collect();
	keys.sort();
	keys.dedup();
	let yaml_key = "multilanguage/message.yaml";
	let mut yaml_pick: Option<(Option<PathBuf>, Option<PathBuf>)> = None;
	if keys.iter().any(|k| k == yaml_key) {
		let mut all: Vec<PathBuf> = en.get(yaml_key).cloned().unwrap_or_default();
		all.extend(ru.get(yaml_key).cloned().unwrap_or_default());
		all.retain(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase().starts_with("message_en")).unwrap_or(false));
		let mut scored: Vec<(f64, PathBuf)> = all.into_iter().map(|p| (cyr_ratio(&p), p)).collect();
		scored.sort_by(|a, b| a.0.total_cmp(&b.0));
		let e = scored.first().filter(|x| x.0 < 0.3).map(|x| x.1.clone());
		let r = scored.last().filter(|x| x.0 >= 0.3).map(|x| x.1.clone());
		yaml_pick = Some((e, r));
	}
	let total = keys.len();
	let results: std::sync::Mutex<Vec<(String, cache::Entry)>> = std::sync::Mutex::new(Vec::new());
	let next = std::sync::atomic::AtomicUsize::new(0);
	let done = std::sync::atomic::AtomicUsize::new(0);
	let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 8);
	std::thread::scope(|s| {
		for _ in 0..threads {
			s.spawn(|| loop {
				let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
				let Some(key) = keys.get(i) else { break };
				let (ep, rp) = if key == yaml_key {
					yaml_pick.clone().unwrap_or((None, None))
				} else {
					(en.get(key).and_then(|v| v.first().cloned()), ru.get(key).and_then(|v| v.first().cloned()))
				};
				let (se, sr) = (cache::sig(&ep), cache::sig(&rp));
				if let Some(c) = old.get(key).filter(|c| c.se == se && c.sr == sr) {
					results.lock().unwrap().push((key.clone(), c.clone()));
					let d = done.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
					progress(d, total);
					continue;
				}
				let (pairs, warn, stat) = pair_files(key, &ep, &rp, key.starts_with("table/") || key == yaml_key);
				results.lock().unwrap().push((key.clone(), cache::Entry { se, sr, pairs, warn, stat }));
				let d = done.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
				progress(d, total);
			});
		}
	});
	let mut res = results.into_inner().unwrap();
	res.sort_by(|a, b| a.0.cmp(&b.0));
	if let Some(f) = &cache_file {
		cache::write(f, &res);
	}
	let mut data = Data::default();
	let mut scenes: Vec<(usize, usize)> = Vec::new();
	for (k, e) in res {
		data.warnings.extend(e.warn);
		if let Some(s) = e.stat.filter(|_| !k.starts_with("table/") && k != yaml_key) {
			scenes.push(s);
		}
		if e.pairs.is_empty() {
			continue;
		}
		let s = data.pairs.len();
		data.pairs.extend(e.pairs);
		data.files.insert(k, (s, data.pairs.len()));
	}
	data.game_warning = game_mismatch(en_dir, ru_dir, &data, &scenes);
	data
}

pub fn check_files(files: &[(String, PathBuf, PathBuf)]) -> Vec<String> {
	let mut d = Data::default();
	for (rel, en, ru) in files {
		let key = file_key(rel).unwrap_or_else(|| rel.clone());
		let (pairs, warn, _) = pair_files(&key, &Some(en.clone()), &Some(ru.clone()), key.starts_with("table/") || key.starts_with("multilanguage/"));
		d.warnings.extend(warn);
		let s = d.pairs.len();
		d.pairs.extend(pairs);
		d.files.insert(key, (s, d.pairs.len()));
	}
	let r = super::checks::run(&d);
	let mut out: Vec<String> = r
		.findings
		.iter()
		.filter(|f| super::checks::cats()[f.cat].2 == 0)
		.map(|f| match d.pairs.get(f.pair) {
			Some(p) => format!("{} · {}: {}", p.file, p.ru.as_ref().map_or(p.id.clone(), |s| lformat!("строка {}", "line {}", s.line)), f.what),
			None => f.what.clone(),
		})
		.collect();
	out.dedup();
	out
}
fn pair_files(key: &str, ep: &Option<PathBuf>, rp: &Option<PathBuf>, by_id: bool) -> (Vec<Pair>, Vec<String>, Option<(usize, usize)>) {
	let mut warn = Vec::new();
	let mut rd = |p: &Option<PathBuf>| -> Vec<Item> {
		match p {
			Some(p) => read_any(p).unwrap_or_else(|e| {
				if !e.contains(tr("не файл диалогов", "not a dialogue file")) || std::env::var("KC_DEBUG").is_ok() {
					warn.push(format!("{}: {e}", p.display()));
				}
				Vec::new()
			}),
			None => Vec::new(),
		}
	};
	let a = rd(ep);
	let b = rd(rp);
	let mut pairs = Vec::new();
	let mut stat = None;
	if !(a.is_empty() && b.is_empty()) {
		let ali = if by_id {
			let mut pos: HashMap<&str, std::collections::VecDeque<usize>> = HashMap::new();
			for (i, x) in b.iter().enumerate() {
				pos.entry(x.key.as_str()).or_default().push_back(i);
			}
			let mut used = vec![false; b.len()];
			let mut v: Vec<(Option<usize>, Option<usize>)> = Vec::new();
			for (i, x) in a.iter().enumerate() {
				let j = pos.get_mut(x.key.as_str()).and_then(|q| q.pop_front());
				if let Some(j) = j {
					used[j] = true;
				}
				v.push((Some(i), j));
			}
			v.extend(used.iter().enumerate().filter(|(_, u)| !**u).map(|(j, _)| (None, Some(j))));
			v
		} else {
			align(&a, &b)
		};
		if !a.is_empty() && !b.is_empty() {
			let same = ali.iter().filter(|(i, j)| matches!((i, j), (Some(i), Some(j)) if a[*i].key == b[*j].key)).count();
			stat = Some((a.len().max(b.len()), same));
		}
		for (i, j) in ali {
			let e = i.map(|i| &a[i]);
			let r = j.map(|j| &b[j]);
			let service = match (e, r) {
				(Some(x), _) => identifier(&x.text),
				(None, Some(y)) => identifier(&y.text),
				(None, None) => continue,
			};
			let side = |x: &Item, p: &Option<PathBuf>| Side { text: x.text.clone(), path: p.clone().unwrap_or_default(), line: x.line };
			pairs.push(Pair {
				file: key.to_string(),
				id: r.or(e).map(|x| x.id.clone()).unwrap_or_default(),
				en_key: e.map(|x| search_key(&x.text)).unwrap_or_default(),
				ru_key: r.map(|x| search_key(&x.text)).unwrap_or_default(),
				en: e.map(|x| side(x, ep)),
				ru: r.map(|x| side(x, rp)),
				who: r.or(e).and_then(|x| speaker_of(&x.key)),
				service,
			});
		}
	}
	(pairs, warn, stat)
}

fn fnv64(d: &[u8]) -> u64 {
	let mut h: u64 = 0xcbf29ce484222325;
	for &b in d {
		h ^= b as u64;
		h = h.wrapping_mul(0x100000001b3);
	}
	h
}

mod cache {
	use super::{search_key, Pair, Side};
	use std::collections::HashMap;
	use std::path::{Path, PathBuf};

	pub type Sig = Option<(String, u64, u64)>;

	#[derive(Clone)]
	pub struct Entry {
		pub se: Sig,
		pub sr: Sig,
		pub pairs: Vec<Pair>,
		pub warn: Vec<String>,
		pub stat: Option<(usize, usize)>,
	}

	pub fn sig(p: &Option<PathBuf>) -> Sig {
		let p = p.as_ref()?;
		let m = std::fs::metadata(p).ok()?;
		let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_nanos() as u64).unwrap_or(0);
		Some((p.to_string_lossy().into_owned(), m.len(), t))
	}

	const MAGIC: &[u8; 8] = b"KCCACHE1";

	fn exe_sig() -> (u64, u64) {
		std::env::current_exe().ok().and_then(|p| sig(&Some(p))).map(|s| (s.1, s.2)).unwrap_or((0, 0))
	}

	struct W(Vec<u8>);
	impl W {
		fn n(&mut self, v: u64) {
			self.0.extend_from_slice(&v.to_le_bytes());
		}
		fn s(&mut self, s: &str) {
			self.n(s.len() as u64);
			self.0.extend_from_slice(s.as_bytes());
		}
		fn sig(&mut self, s: &Sig) {
			match s {
				Some((p, a, b)) => {
					self.n(1);
					self.s(p);
					self.n(*a);
					self.n(*b);
				}
				None => self.n(0),
			}
		}
		fn side(&mut self, s: &Option<Side>) {
			match s {
				Some(x) => {
					self.n(1);
					self.s(&x.text);
					self.s(&x.path.to_string_lossy());
					self.n(x.line as u64);
				}
				None => self.n(0),
			}
		}
	}

	struct R<'a>(&'a [u8], usize);
	impl R<'_> {
		fn n(&mut self) -> Option<u64> {
			let b = self.0.get(self.1..self.1 + 8)?;
			self.1 += 8;
			Some(u64::from_le_bytes(b.try_into().ok()?))
		}
		fn s(&mut self) -> Option<String> {
			let len = self.n()? as usize;
			let b = self.0.get(self.1..self.1.checked_add(len)?)?;
			self.1 += len;
			String::from_utf8(b.to_vec()).ok()
		}
		fn sig(&mut self) -> Option<Sig> {
			Some(if self.n()? == 1 { Some((self.s()?, self.n()?, self.n()?)) } else { None })
		}
		fn side(&mut self) -> Option<Option<Side>> {
			Some(if self.n()? == 1 { Some(Side { text: self.s()?, path: PathBuf::from(self.s()?), line: self.n()? as usize }) } else { None })
		}
	}

	pub fn read(f: &Path) -> Option<HashMap<String, Entry>> {
		let d = std::fs::read(f).ok()?;
		if d.get(..8)? != MAGIC {
			return None;
		}
		let mut r = R(&d, 8);
		if (r.n()?, r.n()?) != exe_sig() {
			return None;
		}
		let mut m = HashMap::new();
		for _ in 0..r.n()? {
			let key = r.s()?;
			let (se, sr) = (r.sig()?, r.sig()?);
			let mut pairs = Vec::new();
			for _ in 0..r.n()? {
				let (file, id) = (r.s()?, r.s()?);
				let (en, ru) = (r.side()?, r.side()?);
				let service = r.n()? == 1;
				let who = u32::try_from(r.n()?).ok();
				let en_key = en.as_ref().map(|x| search_key(&x.text)).unwrap_or_default();
				let ru_key = ru.as_ref().map(|x| search_key(&x.text)).unwrap_or_default();
				pairs.push(Pair { file, id, en, ru, en_key, ru_key, service, who });
			}
			let mut warn = Vec::new();
			for _ in 0..r.n()? {
				warn.push(r.s()?);
			}
			let stat = if r.n()? == 1 { Some((r.n()? as usize, r.n()? as usize)) } else { None };
			m.insert(key, Entry { se, sr, pairs, warn, stat });
		}
		Some(m)
	}

	pub fn write(f: &Path, res: &[(String, Entry)]) {
		let mut w = W(MAGIC.to_vec());
		let (a, b) = exe_sig();
		w.n(a);
		w.n(b);
		w.n(res.len() as u64);
		for (key, e) in res {
			w.s(key);
			w.sig(&e.se);
			w.sig(&e.sr);
			w.n(e.pairs.len() as u64);
			for p in &e.pairs {
				w.s(&p.file);
				w.s(&p.id);
				w.side(&p.en);
				w.side(&p.ru);
				w.n(p.service as u64);
				w.n(p.who.map_or(u64::MAX, u64::from));
			}
			w.n(e.warn.len() as u64);
			for x in &e.warn {
				w.s(x);
			}
			match e.stat {
				Some((x, y)) => {
					w.n(1);
					w.n(x as u64);
					w.n(y as u64);
				}
				None => w.n(0),
			}
		}
		if let Some(dir) = f.parent() {
			let _ = std::fs::create_dir_all(dir);
		}
		let tmp = f.with_extension("part");
		if std::fs::write(&tmp, &w.0).is_ok() && std::fs::rename(&tmp, f).is_err() {
			let _ = std::fs::remove_file(&tmp);
		}
	}
}

fn game_mismatch(en_dir: &Path, ru_dir: &Path, data: &Data, scenes: &[(usize, usize)]) -> Option<String> {
	let by_dir = |d: &Path| -> Option<&'static str> {
		if ed7().is_some() { crate::scena::detect_title(d).map(|t| t.label()) } else { super::game::detect(d).map(|g| g.name()) }
	};
	match (by_dir(en_dir), by_dir(ru_dir)) {
		(Some(a), Some(b)) if a != b => return Some(lformat!("Английские файлы — из {a}, русские — из {b}.", "English files are from {a}, Russian ones from {b}.")),
		(Some(_), Some(_)) => return None,
		_ => {}
	}
	let is_scene = |k: &str| if ed7().is_some() { k.starts_with("scena/") } else { k.ends_with(".json") };
	let (mut en_files, mut ru_files) = (0usize, 0usize);
	for (k, &(a, b)) in &data.files {
		if is_scene(k) {
			en_files += data.pairs[a..b].iter().any(|p| p.en.is_some()) as usize;
			ru_files += data.pairs[a..b].iter().any(|p| p.ru.is_some()) as usize;
		}
	}
	let common = scenes.len();
	let lines: usize = scenes.iter().map(|s| s.0).sum();
	let same: usize = scenes.iter().map(|s| s.1).sum();
	if common >= 3 && lines >= 100 && same * 2 < lines {
		return Some(lformat!("В {common} общих сценах совпали команды скрипта только у {}% реплик — так бывает, когда файлы из разных игр.", "In {common} common scenes, script commands match for only {}% of the lines: this happens when the files are from different games.", same * 100 / lines));
	}
	if common == 0 && en_files >= 20 && ru_files >= 20 {
		return Some(tr("Ни одной общей сцены в английских и русских файлах — так бывает, когда файлы из разных игр.", "The English and Russian files have no scene in common: this happens when the files are from different games.").into());
	}
	None
}


fn has_word(hay: &str, w: &str) -> bool {
	for (i, _) in hay.match_indices(w) {
		let before = hay[..i].chars().next_back();
		let after = hay[i + w.len()..].chars().next();
		let sep = |c: Option<char>| c.map(|c| !c.is_alphanumeric()).unwrap_or(true);
		if sep(before) && sep(after) {
			return true;
		}
	}
	false
}

pub fn identifier(s: &str) -> bool {
	let t = s.trim();
	!t.is_empty() && t.contains('_') && t.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || c == b'-')
}

pub static CASE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn search(d: &Data, query: &str, where_: u8, whole: bool, limit: usize) -> (Vec<usize>, usize) {
	let words: Vec<String> = query.split_whitespace().map(search_key).filter(|w| !w.is_empty()).collect();
	if words.is_empty() {
		return (Vec::new(), 0);
	}
	let case = CASE.load(std::sync::atomic::Ordering::Relaxed);
	let exact: Vec<String> = query.split_whitespace().map(|w| fold(&clean(w, false)).replace('…', "...")).filter(|w| !w.is_empty()).collect();
	let mut out = Vec::new();
	let mut total = 0;
	for (i, p) in d.pairs.iter().enumerate() {
		let found = |hay: &str, ws: &[String]| ws.iter().all(|w| if whole { has_word(hay, w) } else { hay.contains(w.as_str()) });
		let side = |k: &str, s: &Option<Side>| found(k, &words) && (!case || s.as_ref().is_some_and(|s| found(&fold(&clean(&s.text, false)).replace('…', "..."), &exact)));
		let ok = match where_ {
			1 => side(&p.en_key, &p.en),
			2 => side(&p.ru_key, &p.ru),
			_ => side(&p.en_key, &p.en) || side(&p.ru_key, &p.ru),
		};
		if ok {
			total += 1;
			if out.len() < limit {
				out.push(i);
			}
		}
	}
	(out, total)
}

#[derive(Clone, Debug)]
pub struct DictEntry {
	pub en: Vec<String>,
	pub ru: String,
}

fn is_cyr(s: &str) -> bool {
	s.chars().any(|c| ('а'..='я').contains(&c) || ('А'..='Я').contains(&c) || c == 'ё' || c == 'Ё')
}

fn ru_terms(s: &str) -> Vec<String> {
	s.split([',', ';', '/', '|'])
		.flat_map(split_dash_variants)
		.map(|t| fold(&t.trim().to_lowercase()))
		.filter(|t| is_cyr(t) && t.chars().count() >= 2 && t.split_whitespace().count() <= 4)
		.collect()
}

fn split_dash_variants(t: &str) -> Vec<String> {
	let parts: Vec<&str> = t.split(['-', '–', '—']).map(|x| x.trim()).collect();
	let lower: Vec<Vec<char>> = parts.iter().map(|x| x.to_lowercase().chars().collect()).collect();
	let same_stem = lower.iter().all(|w| w.len() >= 2 && lower[0].len() >= 2 && w[..2] == lower[0][..2]);
	if parts.len() > 1 && same_stem && parts.iter().all(|x| x.chars().count() >= 2 && is_cyr(x) && x.split_whitespace().count() == 1) {
		parts.iter().map(|x| x.to_string()).collect()
	} else {
		vec![t.to_string()]
	}
}

fn en_terms(s: &str) -> Vec<String> {
	s.split([',', ';', '/', '|'])
		.map(|t| { let t = fold(&t.trim().to_lowercase()); t.strip_suffix("'s").or_else(|| t.strip_suffix("’s")).or_else(|| t.strip_suffix("\"s")).map(|x| x.to_string()).unwrap_or(t) })
		.filter(|t| !t.is_empty() && !is_cyr(t) && t.chars().any(|c| c.is_ascii_alphabetic()))
		.collect()
}

fn push(out: &mut Vec<DictEntry>, en: &[String], ru: &str) {
	let mut terms = ru_terms(ru);
	for t in ru.split([',', ';', '/', '|']).map(|t| fold(&t.trim().to_lowercase())) {
		if !is_cyr(&t) && t.chars().count() >= 2 && en.contains(&t) && !terms.contains(&t) {
			terms.push(t);
		}
	}
	for r in terms {
		if let Some(e) = out.iter_mut().find(|e| e.ru == r) {
			for x in en {
				if !e.en.contains(x) {
					e.en.push(x.clone());
				}
			}
		} else {
			out.push(DictEntry { en: en.to_vec(), ru: r });
		}
	}
}

pub fn load_dict(path: &Path) -> Result<Vec<DictEntry>, String> {
	let src = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
	let text = String::from_utf8(src.clone()).unwrap_or_else(|_| src.iter().map(|&b| cp1251(b)).collect());
	let body = text.trim_start_matches('\u{feff}').trim_start();
	let mut out: Vec<DictEntry> = Vec::new();
	if body.starts_with('{') || body.starts_with('[') {
		let clean_body = strip_json_comments(body);
		let v: serde_json::Value = serde_json::from_str(&clean_body).map_err(|e| lformat!("ошибка в JSON словаря: {e}", "error in the dictionary JSON: {e}"))?;
		fn walk(v: &serde_json::Value, en: &[String], out: &mut Vec<DictEntry>) {
			match v {
				serde_json::Value::String(s) => push(out, en, s),
				serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, en, out)),
				serde_json::Value::Object(o) => {
					let mut sib: Vec<String> = en.to_vec();
					for (k, x) in o {
						let en_field = ["en", "eng", "english", "source", "original", "term", "word"].contains(&k.to_lowercase().as_str());
						if let (true, serde_json::Value::String(s)) = (en_field, x) {
							sib.extend(en_terms(s));
						}
					}
					for (k, x) in o {
						let kl = k.to_lowercase();
						if kl.starts_with('_') || kl.starts_with('#') || ["comment", "comments", "note", "notes", "коммент", "комментарий", "примечание"].contains(&kl.as_str()) {
							continue;
						}
						let mut e = sib.clone();
						let ke = en_terms(k);
						let field = ["en", "ru", "eng", "rus", "english", "russian", "source", "target", "original", "translation", "term", "word", "note", "comment", "id", "key", "value"]
							.contains(&k.to_lowercase().as_str());
						if !field {
							e.extend(ke);
						}
						if is_cyr(k) {
							push(out, &e, k);
						}
						if x.is_string() && ["en", "eng", "english", "source", "original", "term", "word"].contains(&kl.as_str()) {
							continue;
						}
						walk(x, &e, out);
					}
				}
				_ => {}
			}
		}
		walk(&v, &[], &mut out);
	} else {
		for l in body.lines() {
			let l = l.trim();
			if l.is_empty() || l.starts_with('#') || l.starts_with(';') || l.starts_with("//") {
				continue;
			}
			match l.split_once('=').or_else(|| l.split_once(':')).or_else(|| l.split_once('\t')).or_else(|| l.split_once('→')).or_else(|| l.split_once(" - ")) {
				Some((a, b)) => {
					let (en, ru) = if is_cyr(a) && !is_cyr(b) { (b, a) } else { (a, b) };
					push(&mut out, &en_terms(en), ru);
				}
				None => {
					if l.split_whitespace().count() <= 2 && !l.contains('(') {
						push(&mut out, &[], l)
					}
				}
			}
		}
	}
	Ok(out)
}

pub fn dict_from_books<'a>(books: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Result<Vec<DictEntry>, String> {
	use calamine::{Data, Reader, Xlsx};
	fn text(d: &Data) -> String {
		match d {
			Data::String(s) => s.trim().to_string(),
			Data::Bool(b) => if *b { "TRUE".into() } else { String::new() },
			Data::Empty => String::new(),
			x => x.to_string(),
		}
	}
	let mut out: Vec<DictEntry> = Vec::new();
	let mut used = 0;
	for (name, data) in books {
		let stem = name.rsplit('/').next().unwrap_or(name);
		if !stem.to_lowercase().ends_with(".xlsx") || stem.starts_with('(') {
			continue;
		}
		let mut wb: Xlsx<_> = calamine::open_workbook_from_rs(std::io::Cursor::new(data)).map_err(|e| format!("{stem}: {e}"))?;
		let Some(Ok(range)) = wb.worksheet_range_at(0) else { continue };
		let rows: Vec<Vec<String>> = range.rows().map(|r| r.iter().map(text).collect()).collect();
		let Some(h) = rows.iter().take(10).position(|r| r.iter().any(|c| c.contains("Английский") || c.contains("English"))) else { continue };
		let find = |f: &dyn Fn(&str) -> bool| rows[h].iter().position(|c| f(c));
		let (Some(en), Some(ru)) = (find(&|c| c.contains("Английский") || c.contains("English")), find(&|c| c.starts_with("Русский") || c.starts_with("Итоговый вариант") || c.starts_with("Russian"))) else { continue };
		let fin = find(&|c| c.contains("Финальный") || c.contains("Final"));
		used += 1;
		for r in &rows[h + 1..] {
			let get = |i: usize| r.get(i).map(|s| s.as_str()).unwrap_or("");
			if let Some(f) = fin {
				if !matches!(get(f), "TRUE" | "☑" | "✓") {
					continue;
				}
			}
			let (e, t) = (get(en), get(ru));
			if t.is_empty() || t.contains("??") {
				continue;
			}
			let e = e.split('[').next().unwrap_or("").replace('\n', ",");
			push(&mut out, &en_terms(&e), &t.replace('\n', ","));
		}
	}
	if used == 0 {
		return Err(tr("в словаре нет ни одной вкладки с заголовками «Английский» и «Русский»", "the dictionary has no tab with 'English' and 'Russian' headers").into());
	}
	Ok(out)
}

fn name_words(phrase: &str) -> Vec<String> {
	phrase
		.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '\''))
		.map(|w| w.trim_matches(|c| c == '-' || c == '\''))
		.filter(|w| w.chars().count() >= 2)
		.map(|w| w.to_string())
		.collect()
}

pub fn terms_for_query(dict: &[DictEntry], query: &str) -> (Vec<String>, Vec<String>) {
	let q = fold(&query.trim().to_lowercase());
	let mut en: Vec<String> = Vec::new();
	let mut ru: Vec<String> = Vec::new();
	for w in name_words(&q) {
		if is_cyr(&w) {
			for e in dict {
				for p in name_words(&e.ru) {
					let hit = p == w || (w.starts_with(p.as_str()) && p.chars().count() >= 3);
					if hit && !ru.contains(&p) {
						ru.push(p);
					}
				}
			}
		} else {
			for e in dict {
				for x in &e.en {
					for p in name_words(x) {
						if p == w && !en.contains(&p) {
							en.push(p);
						}
					}
				}
				if !is_cyr(&e.ru) {
					for p in name_words(&e.ru) {
						if p == w && !ru.contains(&p) {
							ru.push(p);
						}
					}
				}
			}
		}
	}
	en.sort_by_key(|t| std::cmp::Reverse(t.chars().count()));
	ru.sort_by_key(|t| std::cmp::Reverse(t.chars().count()));
	(en, ru)
}

pub fn pair_terms_for_query(dict: &[DictEntry], query: &str) -> (Vec<String>, Vec<String>) {
	let (mut en, mut ru) = terms_for_query(dict, query);
	let q = fold(&query.trim().to_lowercase());
	for w in name_words(&q) {
		if is_cyr(&w) {
			for e in dict {
				let hit = name_words(&e.ru).iter().any(|p| *p == w || (w.starts_with(p.as_str()) && p.chars().count() >= 3));
				if hit {
					for x in &e.en {
						for p in name_words(x) {
							if !en.contains(&p) {
								en.push(p);
							}
						}
					}
				}
			}
		}
	}
	for r in ru_words_for_query(dict, query) {
		if !is_cyr(&r) || ru.contains(&r) {
			continue;
		}
		ru.push(r);
	}
	en.sort_by_key(|t| std::cmp::Reverse(t.chars().count()));
	ru.sort_by_key(|t| std::cmp::Reverse(t.chars().count()));
	(en, ru)
}

pub fn ru_words_for_query(dict: &[DictEntry], query: &str) -> Vec<String> {
	let q = fold(&query.trim().to_lowercase());
	let mut out: Vec<String> = Vec::new();
	for w in name_words(&q) {
		if is_cyr(&w) {
			if !out.contains(&w) {
				out.push(w);
			}
			continue;
		}
		for e in dict {
			let ru_w = name_words(&e.ru);
			for x in &e.en {
				let en_w = name_words(x);
				if let Some(pos) = en_w.iter().position(|p| *p == w) {
					let pick: Vec<String> = if en_w.len() == ru_w.len() { vec![ru_w[pos].clone()] } else { ru_w.clone() };
					for r in pick {
						if !out.contains(&r) {
							out.push(r);
						}
					}
				}
			}
		}
	}
	out
}

pub fn count_ru(d: &Data, rows: &[usize], word: &str) -> (usize, usize) {
	count_ru_any(d, rows, &[word.to_string()])
}

pub fn count_ru_any(d: &Data, rows: &[usize], words: &[String]) -> (usize, usize) {
	let (mut times, mut lines) = (0, 0);
	for &i in rows {
		if let Some(r) = &d.pairs[i].ru {
			let n = highlights(&clean(&r.text, false), words).len();
			if n > 0 {
				times += n;
				lines += 1;
			}
		}
	}
	(times, lines)
}

fn has_word_at(text: &[char], w: &[char], whole: bool) -> bool {
	if w.is_empty() || w.len() > text.len() {
		return false;
	}
	for i in 0..=text.len() - w.len() {
		if (i == 0 || !text[i - 1].is_alphanumeric()) && text[i..i + w.len()] == *w {
			if !whole {
				return true;
			}
			let next = text.get(i + w.len());
			let after = text.get(i + w.len() + 1);
			if next.map(|c| !c.is_alphanumeric()).unwrap_or(true) || (matches!(next, Some('\'') | Some('"')) && after == Some(&'s')) || next == Some(&'s') && after.map(|c| !c.is_alphanumeric()).unwrap_or(true) {
				return true;
			}
		}
	}
	false
}

fn fold_chars(s: &str) -> Vec<char> {
	fold(&clean(s, false).to_lowercase()).chars().collect()
}

pub static EXACT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn exact() -> bool {
	EXACT.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn dict_status(en: &str, ru: &str, dict: &[DictEntry]) -> u8 {
	if dict.is_empty() {
		return 0;
	}
	let (any, failed) = DictIndex::new(dict).check(en, ru);
	if !failed.is_empty() { 2 } else if any { 1 } else { 0 }
}

pub struct DictIndex {
	terms: Vec<(String, Vec<String>)>,
	pos: HashMap<String, usize>,
	by_first: HashMap<String, Vec<usize>>,
	always: Vec<usize>,
}

impl DictIndex {
	pub fn new(dict: &[DictEntry]) -> DictIndex {
		let mut terms: Vec<(String, Vec<String>)> = Vec::new();
		let mut pos: HashMap<String, usize> = HashMap::new();
		for d in dict {
			for x in &d.en {
				match pos.get(x.as_str()) {
					Some(&k) => {
						let v = &mut terms[k].1;
						if !v.contains(&d.ru) {
							v.push(d.ru.clone())
						}
					}
					None => {
						pos.insert(x.clone(), terms.len());
						terms.push((x.clone(), vec![d.ru.clone()]));
					}
				}
			}
		}
		let mut by_first: HashMap<String, Vec<usize>> = HashMap::new();
		let mut always = Vec::new();
		for (k, (t, _)) in terms.iter().enumerate() {
			let first = t.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("");
			if first.is_empty() {
				always.push(k);
			} else {
				by_first.entry(first.to_string()).or_default().push(k);
			}
		}
		DictIndex { terms, pos, by_first, always }
	}

	pub fn check(&self, en: &str, ru: &str) -> (bool, Vec<(String, Vec<String>)>) {
		let e = fold_chars(en);
		let r = fold_chars(ru);
		let has_ru = |v: &str| has_word_at(&r, &v.chars().collect::<Vec<_>>(), exact());
		let single = |w: &str| -> Vec<String> { self.pos.get(w).map(|&k| self.terms[k].1.clone()).unwrap_or_default() };
		let text: String = e.iter().collect();
		let mut cand: Vec<usize> = self.always.clone();
		for w in text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
			for key in [Some(w), w.strip_suffix('s')].into_iter().flatten() {
				if let Some(v) = self.by_first.get(key) {
					cand.extend(v);
				}
			}
		}
		cand.sort_unstable();
		cand.dedup();
		let mut any = false;
		let mut failed = Vec::new();
		for k in cand {
			let (t, variants) = &self.terms[k];
			if !has_word_at(&e, &t.chars().collect::<Vec<_>>(), true) {
				continue;
			}
			any = true;
			if variants.iter().any(|v| has_ru(v)) {
				continue;
			}
			let tw = name_words(t);
			if tw.len() > 1 {
				let ok = tw.iter().enumerate().all(|(pos, w)| {
					let mut v: Vec<String> = variants.iter().filter_map(|x| {
						let xw = name_words(x);
						(xw.len() == tw.len()).then(|| xw[pos].clone())
					}).collect();
					v.extend(single(w));
					!v.is_empty() && v.iter().any(|x| has_ru(x))
				});
				if ok {
					continue;
				}
			}
			failed.push((t.clone(), variants.clone()));
		}
		(any, failed)
	}
}

fn strip_json_comments(src: &str) -> String {
	let mut out = String::with_capacity(src.len());
	let mut in_str = false;
	let mut esc = false;
	let mut chars = src.chars().peekable();
	let mut line_start = true;
	while let Some(c) = chars.next() {
		if in_str {
			out.push(c);
			if esc {
				esc = false;
			} else if c == '\\' {
				esc = true;
			} else if c == '"' {
				in_str = false;
			}
			continue;
		}
		let comment_line = (c == '/' && chars.peek() == Some(&'/')) || (c == '#' && line_start);
		if comment_line {
			while let Some(&n) = chars.peek() {
				if n == '\n' {
					break;
				}
				chars.next();
			}
			continue;
		}
		if c == '/' && chars.peek() == Some(&'*') {
			chars.next();
			let mut prev = ' ';
			for n in chars.by_ref() {
				if prev == '*' && n == '/' {
					break;
				}
				prev = n;
			}
			continue;
		}
		if c == '"' {
			in_str = true;
		}
		if c == '\n' {
			line_start = true;
		} else if !c.is_whitespace() {
			line_start = false;
		}
		out.push(c);
	}
	let mut res = String::with_capacity(out.len());
	let v: Vec<char> = out.chars().collect();
	let mut in_s = false;
	let mut e = false;
	for (i, &c) in v.iter().enumerate() {
		if in_s {
			if e { e = false } else if c == '\\' { e = true } else if c == '"' { in_s = false }
			res.push(c);
			continue;
		}
		if c == '"' {
			in_s = true;
		}
		if c == ',' {
			let next = v[i + 1..].iter().find(|x| !x.is_whitespace());
			if matches!(next, Some('}') | Some(']')) {
				continue;
			}
		}
		res.push(c);
	}
	res
}

fn cp1251(b: u8) -> char {
	match b {
		0..=0x7f => b as char,
		0xc0..=0xff => char::from_u32(0x410 + (b as u32 - 0xc0)).unwrap_or('?'),
		0xa8 => 'Ё',
		0xb8 => 'ё',
		_ => '?',
	}
}

pub fn highlights(text: &str, dict: &[String]) -> Vec<(usize, usize)> {
	if dict.is_empty() {
		return Vec::new();
	}
	let low: Vec<char> = text.chars().map(|c| fold(&c.to_lowercase().next().unwrap_or(c).to_string()).chars().next().unwrap_or(c)).collect();
	let mut marks = vec![false; low.len()];
	for t in dict {
		let tc: Vec<char> = t.chars().collect();
		if tc.len() > low.len() {
			continue;
		}
		for i in 0..=low.len() - tc.len() {
			let starts_word = i == 0 || !low[i - 1].is_alphanumeric();
			let ends_ok = || {
				if !exact() {
					return true;
				}
				let next = low.get(i + tc.len());
				let after = low.get(i + tc.len() + 1);
				next.map(|c| !c.is_alphanumeric()).unwrap_or(true) || (matches!(next, Some('\'') | Some('"')) && after == Some(&'s'))
			};
			if starts_word && low[i..i + tc.len()] == tc[..] && ends_ok() {
				for m in &mut marks[i..i + tc.len()] {
					*m = true;
				}
			}
		}
	}
	let mut out = Vec::new();
	let mut i = 0;
	while i < marks.len() {
		if marks[i] {
			let s = i;
			while i < marks.len() && marks[i] {
				i += 1;
			}
			out.push((s, i));
		} else {
			i += 1;
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn tmp(name: &str) -> PathBuf {
		let d = std::env::temp_dir().join(format!("kc_test_{name}_{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&d);
		std::fs::create_dir_all(&d).unwrap();
		d
	}

	#[test]
	fn dict_text_and_status() {
		let d = tmp("dict");
		let f = d.join("dict.txt");
		std::fs::write(&f, "ARC EN CIEL:Арк-ен-Сиэль\nLeman State:Автономия Леман, Автономии Леман\nS-Boost:S-буст, S-буста\nShizuna:Шизуна-Шизуны-Шизуне\n# комментарий\n").unwrap();
		let dict = load_dict(&f).unwrap();
		let has = |ru: &str, en: &str| dict.iter().any(|e| e.ru == ru && e.en.iter().any(|x| x == en));
		assert!(has("арк-ен-сиэль", "arc en ciel"));
		assert!(has("автономия леман", "leman state") && has("автономии леман", "leman state"));
		assert!(has("s-буст", "s-boost") && has("s-буста", "s-boost"));
		assert!(has("шизуна", "shizuna") && has("шизуны", "shizuna") && has("шизуне", "shizuna"));
		assert_eq!(dict_status("The ARC EN CIEL of Leman State", "Арк-ен-Сиэль Автономии Леман", &dict), 1);
		assert_eq!(dict_status("Shizuna came", "Пришла Сизуна", &dict), 2);
		assert_eq!(dict_status("Shizuna came", "Пришла Шизуна", &dict), 1);
		assert_eq!(dict_status("Nothing here", "Ничего", &dict), 0);
		let _ = std::fs::remove_dir_all(&d);
	}

	#[test]
	fn dict_from_sheets() {
		let Ok(dir) = std::env::var("SHEETS_DIR") else { return };
		let files: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir).unwrap().flatten()
			.map(|e| (e.file_name().to_string_lossy().to_string(), std::fs::read(e.path()).unwrap())).collect();
		let dict = dict_from_books(files.iter().map(|(n, b)| (n.as_str(), b.as_slice()))).unwrap();
		println!("entries: {}", dict.len());
		for e in dict.iter().take(40) {
			println!("{:?} -> {}", e.en, e.ru);
		}
		assert!(dict.iter().any(|e| e.ru == "ллойд баннингс" && e.en.iter().any(|x| x == "lloyd bannings")));
	}

	#[test]
	fn align_inserted_line() {
		let it = |k: &str| Item { key: k.into(), id: String::new(), text: String::new(), line: 0 };
		let a = vec![it("a"), it("b"), it("c")];
		let b = vec![it("a"), it("x"), it("b"), it("c")];
		assert_eq!(align(&a, &b), vec![(Some(0), Some(0)), (None, Some(1)), (Some(1), Some(2)), (Some(2), Some(3))]);
	}

	fn scene(dir: &Path, name: &str, code: &str, text: &str) {
		let items: Vec<String> = (1..=40).map(|i| format!(r#"{{"code":"{code}({i}, $1);","text":["{text} {i}"]}}"#)).collect();
		let p = dir.join("script_en").join(format!("{name}.json"));
		std::fs::create_dir_all(p.parent().unwrap()).unwrap();
		std::fs::write(p, format!(r#"{{"kiseki":"kai-scena","script":[{}]}}"#, items.join(","))).unwrap();
	}

	#[test]
	fn game_check_and_cache() {
		let root = tmp("games");
		for n in ["c0000", "c0001", "c0002"] {
			scene(&root.join("en"), n, "talk", "Hello");
			scene(&root.join("ru"), n, "talk", "Привет");
			scene(&root.join("ru2"), n, "other", "Привет");
		}
		let same = load(&root.join("en"), &root.join("ru"), &|_, _| {});
		assert_eq!(same.pairs.len(), 120);
		assert!(same.game_warning.is_none());
		let other = load(&root.join("en"), &root.join("ru2"), &|_, _| {});
		assert!(other.game_warning.is_some(), "different scenes were not recognized as different games");
		let c = root.join("cache");
		let a = load_cached(&root.join("en"), &root.join("ru"), &|_, _| {}, Some(&c));
		assert_eq!(std::fs::read_dir(&c).unwrap().count(), 1);
		let b = load_cached(&root.join("en"), &root.join("ru"), &|_, _| {}, Some(&c));
		assert_eq!(a.pairs.len(), b.pairs.len());
		assert!(a.pairs.iter().zip(&b.pairs).all(|(x, y)| x.id == y.id && x.file == y.file && x.service == y.service && x.who == y.who && x.en_key == y.en_key && x.ru_key == y.ru_key
			&& x.en.as_ref().map(|s| (&s.text, s.line)) == y.en.as_ref().map(|s| (&s.text, s.line))
			&& x.ru.as_ref().map(|s| (&s.text, s.line)) == y.ru.as_ref().map(|s| (&s.text, s.line))));
		let _ = std::fs::remove_dir_all(&root);
	}
}
