#![allow(dead_code)]
#[allow(unused_imports)]
use crossbell_tool::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crossbell_tool::lang::tr;

use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::gui_common::*;
use crossbell_tool::kai::checks;
use crossbell_tool::kai::compare::{self, Data};
use std::collections::{HashMap, HashSet};


fn ign_file() -> PathBuf {
	let name = if compare::ed7().is_some() { "KisekiCompare_hidden.txt" } else { "KaiCompare_hidden.txt" };
	let new = cache_dir().join(name);
	let old = exe_dir().join(name);
	if old.is_file() && !new.is_file() {
		let _ = std::fs::create_dir_all(cache_dir());
		if std::fs::rename(&old, &new).is_err() && std::fs::copy(&old, &new).is_ok() {
			let _ = std::fs::remove_file(&old);
		}
	}
	new
}

fn cache_dir() -> PathBuf {
	exe_dir().join("CrossbellCompare_cache")
}

const LIMIT: usize = 5000;

pub fn read_dict(path: &std::path::Path) -> Result<Vec<compare::DictEntry>, String> {
	if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("dat")) {
		return compare::load_dict(path);
	}
	let raw = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
	let books = crate::kai_buttons::pack::parse(&raw).ok_or_else(|| lformat!("{}: файл повреждён или это не словарь", "{}: the file is damaged or is not a dictionary", path.display()))?;
	let mut names: Vec<&String> = books.keys().collect();
	names.sort();
	compare::dict_from_books(names.into_iter().map(|n| (n.as_str(), books[n].as_slice())))
}

#[derive(Default)]
struct Loading {
	done: AtomicUsize,
	total: AtomicUsize,
	finished: AtomicBool,
	result: Mutex<Option<(Data, checks::Report)>>,
}

fn clamp_safe(v: f32, min: f32, max: f32) -> f32 {
	v.min(max).max(min)
}

const FACE: f32 = 52.0;

fn face_index(pack: &'static [u8]) -> HashMap<u32, &'static [u8]> {
	let mut out = HashMap::new();
	if pack.len() < 12 || &pack[..6] != b"KFACE1" {
		return out;
	}
	let u = |o: usize| pack.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
	let mut o = 12;
	for _ in 0..u(8).unwrap_or(0) {
		let (Some(code), Some(len)) = (u(o), u(o + 4)) else { break };
		let Some(png) = pack.get(o + 8..o + 8 + len as usize) else { break };
		out.insert(code, png);
		o += 8 + len as usize;
	}
	out
}

fn face_code(text: &str) -> Option<u32> {
	code_with(text, b'F')
}

fn voice_code(text: &str) -> Option<u32> {
	code_with(text, b'V')
}

fn code_with(text: &str, letter: u8) -> Option<u32> {
	let b = text.as_bytes();
	let mut i = 0;
	while i < b.len() {
		if b[i] == b'#' {
			let mut j = i + 1;
			while j < b.len() && b[j].is_ascii_digit() {
				j += 1;
			}
			if j > i + 1 && b.get(j) == Some(&letter) {
				return text[i + 1..j].parse().ok();
			}
			i = j;
		} else {
			i += 1;
		}
	}
	None
}

struct RowLayout {
	h: f32,
	en: Arc<egui::Galley>,
	ru: Arc<egui::Galley>,
	ok: u8,
	ru_icons: Vec<(usize, usize, u32)>,
	en_icons: Vec<(usize, usize, u32)>,
}

#[derive(Clone)]
struct Inline {
	row: usize,
	col: usize,
	text: String,
}

fn plural(n: usize, one: &str, few: &str, many: &str) -> String {
	let (m10, m100) = (n % 10, n % 100);
	let w = if ui_en() { if n == 1 { one } else { many } } else if m10 == 1 && m100 != 11 { one } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) { few } else { many };
	format!("{} {w}", group(n))
}

fn group(n: usize) -> String {
	let s = n.to_string();
	let mut out = String::new();
	for (i, c) in s.chars().enumerate() {
		if i > 0 && (s.len() - i) % 3 == 0 {
			out.push('\u{202f}');
		}
		out.push(c);
	}
	out
}

pub struct App {
	pub pal: Pal,
	settings: Settings,
	en: String,
	ru: String,
	dict_path: PathBuf,
	dict_mtime: Option<std::time::SystemTime>,
	dict: Vec<compare::DictEntry>,
	dict_err: Option<String>,
	data: Option<Data>,
	loading: Option<Arc<Loading>>,
	load_status: String,
	load_warn: usize,

	search: String,
	query: String,
	whole: bool,
	case: bool,
	mark_ok: bool,
	file_sel: usize,
	file_items: Vec<String>,

	rows: Vec<usize>,
	total: usize,
	found: String,
	chip: Option<String>,
	searched: bool,

	layouts: Vec<Option<RowLayout>>,
	layout_key: (u32, u32, bool, u32),
	zoom: f32,
	layout_next: usize,
	terms: (Vec<String>, Vec<String>, Vec<String>),

	col_file: f32,
	col_split: f32,
	selected: Option<usize>,
	click_col: usize,
	inline: Option<Inline>,
	face_idx: HashMap<&'static str, HashMap<u32, &'static [u8]>>,
	face_tex: HashMap<(&'static str, u32), Option<egui::TextureHandle>>,
	open_group: Option<(usize, String)>,
	top_row: usize,
	checks_off: f32,
	restore_row: Option<(bool, String, String)>,
	restore_file: Option<String>,
	search_pending: bool,
	menu: Option<(usize, Pos2)>,
	check_menu: Option<(usize, Pos2)>,
	flash: Option<(String, Instant)>,
	dialog: Dialog<()>,
	scroll_top: bool,
	fullscreen: bool,
	page: usize,
	report: Option<checks::Report>,
	cat: usize,
	kinds: [bool; 3],
	hidden_open: bool,
	auto_anchor: Option<Pos2>,
	auto_since: Option<Instant>,
	bar_set: std::collections::HashMap<&'static str, f32>,
	ignored: HashSet<String>,
	scroll_to: Option<usize>,
	rows_cache: Option<std::rc::Rc<Vec<Row>>>,
	rows_all_cache: Option<std::rc::Rc<Vec<Row>>>,
	game_ok: bool,
	drop_split: f32,
	btn: crate::kai_buttons::Buttons,
	record: std::collections::HashMap<usize, u32>,
	ui_extra: std::collections::HashMap<usize, u32>,
	btn_open: bool,
	btn_sel: Option<u32>,
	btn_codes: Option<std::collections::BTreeMap<u32, (usize, String)>>,
}

impl App {
	pub fn new(cc: &eframe::CreationContext) -> App {
		setup_fonts(&cc.egui_ctx);
		set_dialog_owner(cc);
		egui_extras::install_image_loaders(&cc.egui_ctx);
		let st = if compare::ed7().is_some() { Settings::load("KisekiTool_compare.ini", "KisekiCompare") } else { Settings::load("KaiCompare.ini", "KaiCompare") };
		UI_EN.store(st.get("lang") != Some("ru"), std::sync::atomic::Ordering::Relaxed);
		let pal = Pal::get(st.get("dark") == Some("1"));
		apply_visuals(&cc.egui_ctx, &pal);

		let mut app = App {
			pal,
			en: st.get_or("english", "").to_string(),
			ru: st.get_or("russian", "").to_string(),
			dict_path: PathBuf::new(),
			dict_mtime: None,
			dict: Vec::new(),
			dict_err: None,
			data: None,
			loading: None,
			load_status: String::new(),
			load_warn: 0,
			search: String::new(),
			query: String::new(),
			whole: st.get("whole") == Some("1"),
			case: st.get("case") == Some("1"),
			mark_ok: st.get("dictok") == Some("1"),
			file_sel: 0,
			file_items: vec![tr("(все найденные — через поиск)", "(all found, use search)").into()],
			rows: Vec::new(),
			total: 0,
			found: String::new(),
			chip: None,
			searched: false,
			layouts: Vec::new(),
			layout_key: (0, 0, false, 0),
			zoom: 1.0,
			layout_next: 0,
			terms: Default::default(),
			col_file: 170.0,
			col_split: 0.5,
			selected: None,
			click_col: 2,
			inline: None,
			face_idx: HashMap::new(),
			face_tex: HashMap::new(),
			open_group: None,
			top_row: 0,
			checks_off: 0.0,
			restore_row: None,
			restore_file: None,
			search_pending: false,
			menu: None,
			check_menu: None,
			flash: None,
			dialog: Dialog::None,
			scroll_top: false,
			fullscreen: false,
			page: 0,
			report: None,
			cat: 0,
			kinds: [true; 3],
			hidden_open: false,
			auto_anchor: None,
			auto_since: None,
			bar_set: Default::default(),
			ignored: read_set(&ign_file(), false),
			scroll_to: None,
			rows_cache: None,
			rows_all_cache: None,
			game_ok: false,
			drop_split: f32::MAX,
			record: Default::default(),
			ui_extra: Default::default(),
			btn: crate::kai_buttons::Buttons::new(st.get("btn_on") == Some("1"), st.get("btn_dev").and_then(|v| v.parse().ok()).unwrap_or(0), st.get_or("btn_dir", "").to_string()),
			btn_open: false,
			btn_sel: None,
			btn_codes: None,
			settings: st,
		};
		let dp = app.settings.get_or("dict", "").to_string();
		if !dp.is_empty() {
			app.load_dict(&PathBuf::from(dp), false);
		}
		app.search = app.settings.get_or("last_search", "").to_string();
		app.zoom = app.settings.get("zoom").and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0).clamp(0.7, 2.0);
		cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
		app.page = app.settings.get("last_page").and_then(|v| v.parse().ok()).unwrap_or(0).min(1);
		app.cat = app.settings.get("last_cat").and_then(|v| v.parse().ok()).unwrap_or(0).min(checks::cats().len() - 1);
		app.open_group = app.settings.get("last_group").filter(|g| !g.is_empty()).map(|g| (app.cat, g.to_string()));
		if let Some(o) = app.settings.get("last_checks_scroll").and_then(|v| v.parse::<f32>().ok()) {
			app.bar_set.insert("items", o);
		}
		app.restore_file = app.settings.get("last_file").filter(|s| !s.is_empty()).map(|s| s.to_string());
		app.restore_row = app.settings.get("last_row").and_then(|v| {
			let mut it = v.splitn(3, '|');
			Some((it.next()? == "sel", it.next()?.to_string(), it.next()?.to_string()))
		});
		if app.btn.on {
			app.btn.ensure_loaded();
		}
		if app.en_dir().is_dir() && app.ru_dir().is_dir() {
			app.start_load(&cc.egui_ctx);
		}
		app
	}

	pub fn en_path(&self) -> PathBuf {
		self.en_dir()
	}

	fn en_dir(&self) -> PathBuf {
		PathBuf::from(self.en.trim())
	}
	pub fn ru_dir(&self) -> PathBuf {
		PathBuf::from(self.ru.trim())
	}

	fn save_settings(&mut self) {
		let st = &mut self.settings;
		st.set("english", &self.en);
		st.set("russian", &self.ru);
		st.set("whole", if self.whole { "1" } else { "0" });
		st.set("dictok", if self.mark_ok { "1" } else { "0" });
		st.set("dark", if self.pal.dark { "1" } else { "0" });
		st.set("lang", if ui_en() { "en" } else { "ru" });
		st.set("case", if self.case { "1" } else { "0" });
		st.set("dict", &self.dict_path.to_string_lossy());
		st.set("btn_on", if self.btn.on { "1" } else { "0" });
		st.set("btn_dev", &self.btn.device.to_string());
		st.set("btn_dir", &self.btn.game_dir);
		st.set("last_search", self.search.trim());
		st.set("zoom", &format!("{:.1}", self.zoom));
		st.set("last_page", &self.page.to_string());
		st.set("last_cat", &self.cat.to_string());
		st.set("last_group", self.open_group.as_ref().filter(|(c, _)| *c == self.cat).map(|(_, g)| g.as_str()).unwrap_or(""));
		st.set("last_checks_scroll", &format!("{:.0}", self.checks_off));
		let row = self.selected.map(|i| (true, i)).or_else(|| (!self.rows.is_empty()).then_some((false, self.top_row)));
		let last = match (row, self.data.as_ref()) {
			(Some((sel, i)), Some(d)) => self.rows.get(i).map(|&k| format!("{}|{}|{}", if sel { "sel" } else { "top" }, d.pairs[k].file, d.pairs[k].id)).unwrap_or_default(),
			_ => String::new(),
		};
		st.set("last_row", &last);
		let file = match (self.file_sel, self.data.as_ref()) {
			(n, Some(d)) if n > 0 => d.files.keys().nth(n - 1).cloned().unwrap_or_default(),
			_ => String::new(),
		};
		st.set("last_file", &file);
		st.save();
	}

	fn flash(&mut self, s: &str) {
		self.flash = Some((s.to_string(), Instant::now()));
	}

	fn start_load(&mut self, ctx: &egui::Context) {
		if self.loading.is_some() {
			return;
		}
		let (en, ru) = (self.en_dir(), self.ru_dir());
		if !en.is_dir() || !ru.is_dir() {
			self.dialog = Dialog::Info(if compare::ed7().is_some() {
				tr("Укажите обе папки: слева и справа (японские, английские или русские файлы).\nПодходят папка data игры, папка русификатора или папка проекта (.json, .xlsx).", "Choose both folders, left and right (Japanese, English or Russian files).\nA game data folder, a translation mod's folder or a project folder (.json, .xlsx) will do.").into()
			} else {
				tr("Укажите обе папки: слева и справа — японские, английские или русские файлы (.json, .xlsx, .yaml, декомпилированные CrossbellTool).", "Choose both folders, left and right: Japanese, English or Russian files (.json, .xlsx, .yaml decompiled by CrossbellTool).").into()
			});
			return;
		}
		self.save_settings();
		let l = Arc::new(Loading::default());
		self.loading = Some(l.clone());
		let ctx = ctx.clone();
		let no_cache = ctx.input(|i| i.modifiers.shift);
		let cache_dir = cache_dir();
		if no_cache {
			let keep = ign_file();
			for e in std::fs::read_dir(&cache_dir).into_iter().flatten().flatten() {
				let p = e.path();
				if p == keep {
					continue;
				}
				let _ = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
			}
		}
		std::thread::spawn(move || {
			let lp = l.clone();
			let c2 = ctx.clone();
			let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
				let d = compare::load_cached(&en, &ru, &|d, t| {
					lp.done.store(d, Ordering::SeqCst);
					lp.total.store(t, Ordering::SeqCst);
					if d % 20 == 0 || d == t {
						c2.request_repaint();
					}
				}, Some(&cache_dir));
				let r = checks::run(&d);
				(d, r)
			}))
			.unwrap_or_else(|e| {
				let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
				let d = Data { warnings: vec![lformat!("загрузка прервана внутренней ошибкой: {msg}", "loading was interrupted by an internal error: {msg}")], ..Default::default() };
				let r = checks::run(&d);
				(d, r)
			});
			*l.result.lock().unwrap() = Some(res);
			l.finished.store(true, Ordering::SeqCst);
			ctx.request_repaint();
		});
	}

	fn poll_load(&mut self) {
		let Some(l) = self.loading.clone() else { return };
		if !l.finished.load(Ordering::SeqCst) {
			return;
		}
		self.loading = None;
		self.search_pending = false;
		let (d, report) = l.result.lock().unwrap().take().unwrap_or_default();
		self.file_items = vec![tr("(все найденные — через поиск)", "(all found, use search)").to_string()];
		self.file_items.extend(d.files.iter().map(|(k, (a, b))| format!("{k}   ({})", b - a)));
		self.file_sel = 0;
		self.load_warn = d.warnings.len();
		self.load_status = if d.pairs.is_empty() {
			tr("ничего не найдено — укажите папки проекта CrossbellTool", "nothing found: choose CrossbellTool project folders").into()
		} else {
			format!("{} · {}", plural(d.files.len(), tr("файл", "file"), tr("файла", "files"), tr("файлов", "files")), plural(d.pairs.len(), tr("строка", "line"), tr("строки", "lines"), tr("строк", "lines")))
		};
		for w in &d.warnings {
			eprintln!("{w}");
		}
		self.record = if compare::ed7().is_some() { crate::kai_buttons::record_cells(&d) } else { Default::default() };
		self.ui_extra = if compare::ed7().is_none() {
			let mut m = crate::kai_buttons::sibling_icons(&d);
			m.extend(crate::kai_buttons::cond_icons(&d));
			m
		} else {
			Default::default()
		};
		if compare::ed7().is_none() {
			let en = self.en_path();
			let places: Vec<PathBuf> = [Some(en.clone()), en.parent().map(|p| p.to_path_buf()), en.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf())].into_iter().flatten().collect();
			self.btn.load_kai_items(&places);
		}
		self.data = Some(d);
		self.report = Some(report);
		self.rows_cache = None;
		self.rows_all_cache = None;
		self.game_ok = false;
		self.btn_codes = None;
		self.rows.clear();
		self.selected = None;
		self.inline = None;
		self.menu = None;
		self.scroll_to = None;
		self.searched = false;
		self.found.clear();
		self.chip = None;
		self.reset_layouts();
		if !self.search.trim().is_empty() {
			self.do_search();
		}
		if let (Some(f), Some(d)) = (self.restore_file.take(), self.data.as_ref()) {
			if let Some(n) = d.files.keys().position(|k| *k == f) {
				self.file_sel = n + 1;
				self.show_file();
			}
		}
		if let (Some((sel, f, id)), Some(d)) = (self.restore_row.take(), self.data.as_ref()) {
			if let Some(i) = self.rows.iter().position(|&k| d.pairs[k].file == f && d.pairs[k].id == id) {
				self.scroll_to = Some(i);
				if sel {
					self.selected = Some(i);
				}
			}
		}
	}

	fn load_dict(&mut self, path: &std::path::Path, force: bool) {
		let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
		if !force && self.dict_path == path && self.dict_mtime == mtime && !self.dict.is_empty() {
			return;
		}
		match read_dict(path) {
			Ok(words) => {
				self.dict = words;
				self.dict_path = path.to_path_buf();
				self.dict_mtime = mtime;
				self.dict_err = None;
			}
			Err(e) => {
				self.dict.clear();
				self.dict_path = path.to_path_buf();
				self.dict_err = Some(e.clone());
				if force {
					self.dialog = Dialog::Info(lformat!("Не удалось прочитать словарь:\n{e}", "Could not read the dictionary:\n{e}"));
				}
			}
		}
	}

	fn refresh_dict(&mut self) {
		if !self.dict_path.as_os_str().is_empty() {
			let p = self.dict_path.clone();
			self.load_dict(&p, false);
		}
	}

	fn do_search(&mut self) {
		self.refresh_dict();
		compare::EXACT.store(self.whole, Ordering::Relaxed);
		compare::CASE.store(self.case, Ordering::Relaxed);
		let q = self.search.trim().to_string();
		self.query = q.clone();
		self.file_sel = 0;
		self.inline = None;
		if self.loading.is_some() {
			self.search_pending = true;
			self.rows.clear();
			self.searched = false;
			self.found.clear();
			self.chip = None;
			self.reset_layouts();
			return;
		}
		let Some(d) = self.data.as_ref() else {
			if self.loading.is_none() {
				self.dialog = Dialog::Info(tr("Сначала нажмите «Загрузить».", "Click 'Load' first.").into());
			}
			return;
		};
		let (all, total) = compare::search(d, &q, 0, self.whole, usize::MAX);
		let rows: Vec<usize> = all[..all.len().min(LIMIT)].to_vec();
		let files: std::collections::BTreeSet<&str> = rows.iter().map(|&i| d.pairs[i].file.as_str()).collect();
		self.found = if total == 0 {
			tr("Ничего не найдено", "Nothing found").into()
		} else if total > rows.len() {
			lformat!("Найдено: {} в {} — показаны первые {}", "Found: {} in {}, showing the first {}", plural(total, tr("строка", "line"), tr("строки", "lines"), tr("строк", "lines")), plural(files.len(), tr("файле", "file"), tr("файлах", "files"), tr("файлах", "files")), group(rows.len()))
		} else {
			lformat!("Найдено: {} в {}", "Found: {} in {}", plural(total, tr("строка", "line"), tr("строки", "lines"), tr("строк", "lines")), plural(files.len(), tr("файле", "file"), tr("файлах", "files"), tr("файлах", "files")))
		};
		self.chip = None;
		let words = compare::ru_words_for_query(&self.dict, &q);
		if !words.is_empty() && total > 0 {
			self.chip = Some(if words.len() <= 2 {
				words
					.iter()
					.map(|w| {
						let (t, l) = compare::count_ru(d, &all, w);
						lformat!("«{w}» в русском: {} (в {})", "'{w}' in Russian: {} (in {})", plural(t, tr("раз", "time"), tr("раза", "times"), tr("раз", "time")), plural(l, tr("строке", "line"), tr("строках", "lines"), tr("строках", "lines")))
					})
					.collect::<Vec<_>>()
					.join(" · ")
			} else {
				let each: Vec<String> = words.iter().map(|w| format!("{w} {}", compare::count_ru(d, &all, w).0)).collect();
				let (t, l) = compare::count_ru_any(d, &all, &words);
				lformat!("в русском: {} (в {}) — {}", "in Russian: {} (in {}), {}", plural(t, tr("раз", "time"), tr("раза", "times"), tr("раз", "time")), plural(l, tr("строке", "line"), tr("строках", "lines"), tr("строках", "lines")), each.join(", "))
			});
		}
		self.rows = rows;
		self.total = total;
		self.searched = true;
		self.selected = None;
		self.scroll_top = true;
		self.reset_layouts();
	}

	fn show_file(&mut self) {
		if self.file_sel == 0 {
			return;
		}
		self.refresh_dict();
		self.query = self.search.trim().to_string();
		let Some(d) = self.data.as_ref() else { return };
		if let Some(&(a, b)) = d.files.values().nth(self.file_sel - 1) {
			self.rows = (a..b).collect();
			self.total = b - a;
			self.found = lformat!("{} в файле", "{} in the file", plural(b - a, tr("строка", "line"), tr("строки", "lines"), tr("строк", "lines")));
			self.chip = None;
			self.searched = true;
			self.selected = None;
			self.inline = None;
			self.scroll_top = true;
			self.reset_layouts();
		}
	}

	fn reset_layouts(&mut self) {
		self.layouts = (0..self.rows.len()).map(|_| None).collect();
		self.layout_next = 0;
		let (ten, tru) = compare::pair_terms_for_query(&self.dict, &self.query);
		let hits: Vec<String> = self.query.split_whitespace().map(compare::search_key).filter(|w| !w.is_empty()).collect();
		self.terms = (ten, tru, hits);
	}

	fn row_text(&self, i: usize, col: usize) -> Option<String> {
		let d = self.data.as_ref()?;
		let p = &d.pairs[*self.rows.get(i)?];
		let side = if col == 1 { &p.en } else { &p.ru };
		side.as_ref().map(|s| compare::clean(&s.text, true))
	}

	fn row_file(&self, i: usize) -> Option<String> {
		let d = self.data.as_ref()?;
		let p = &d.pairs[*self.rows.get(i)?];
		Some(display_name(p))
	}

	fn layout_row(&self, ui: &egui::Ui, i: usize, wen: f32, wru: f32) -> RowLayout {
		let p = &self.pal;
		let d = self.data.as_ref().unwrap();
		let pair = &d.pairs[self.rows[i]];
		let icons_on = self.btn.on;
		let rank = {
			let pr = &d.pairs[self.rows[i]];
			let f = pr.file.rsplit('/').next().unwrap_or("");
			if !icons_on || compare::ed7().is_some() {
				None
			} else if f.starts_with("t_skill.") {
				pr.id.strip_prefix("SkillParam#").and_then(|r| r.strip_suffix(".name")).and_then(|k| k.parse::<u32>().ok()).and_then(|k| self.btn.kai_skill_rank.get(&k).cloned())
			} else if f.starts_with("t_item.") {
				pr.id.strip_prefix("ItemTableData#").and_then(|r| r.strip_suffix(".name")).and_then(|k| k.parse::<u32>().ok()).and_then(|k| self.btn.kai_item_tail.get(&k).cloned())
			} else {
				None
			}
		};
		let rec = self.record.get(&self.rows[i]).copied().map(|c| crate::kai_buttons::RECORD_BASE + c).or_else(|| {
			let pr = &d.pairs[self.rows[i]];
			let fname = pr.file.rsplit('/').next().unwrap_or("");
			if compare::ed7().is_some() && fname.starts_with("t_magic") {
				pr.en.as_ref().and_then(|s| self.btn.craft_cell(&s.text))
			} else if compare::ed7().is_none() && fname.starts_with("t_item.") {
				pr.id.strip_prefix("ItemTableData#").and_then(|r| {
					let (k, f) = r.split_once('.')?;
					(f == "name" || f == "description").then_some(())?;
					self.btn.kai_items.get(&k.parse::<u32>().ok()?).copied()
				})
			} else if compare::ed7().is_none() && fname.starts_with("t_skill.") {
				pr.id.strip_prefix("SkillParam#").and_then(|r| {
					let (k, f) = r.split_once('.')?;
					(f == "name" || f.starts_with("description")).then_some(())?;
					self.btn.kai_skills.get(&k.parse::<u32>().ok()?).copied()
				})
			} else if compare::ed7().is_none() && fname.starts_with("t_itemhelp.") && pr.id.starts_with("SkillRangeHelpData#") {
				pr.id.strip_prefix("SkillRangeHelpData#").and_then(|r| {
					let (k, f) = r.split_once('.')?;
					(f == "text1" || f == "text2").then_some(())?;
					self.btn.kai_ranges.get(&k.parse::<u32>().ok()?).copied()
				})
			} else if compare::ed7().is_none() && fname.starts_with("t_itemhelp.") && self.btn.kai_help.contains_key(&pr.id) {
				self.btn.kai_help.get(&pr.id).copied()
			} else if compare::ed7().is_none() {
				crate::kai_buttons::ui_icon(&pr.file, &pr.id).or_else(|| self.ui_extra.get(&self.rows[i]).copied())
			} else {
				None
			}
		}).filter(|_| icons_on);
		let make = |side: &Option<compare::Side>, dict_terms: &[String], w: f32| -> (Arc<egui::Galley>, Vec<(usize, usize, u32)>) {
			let mut icons: Vec<(usize, usize, u32)> = Vec::new();
			let mut job = egui::text::LayoutJob::default();
			job.wrap.max_width = w.max(20.0);
			let z = self.zoom;
			let fmt = |bg: Color32, color: Color32| egui::TextFormat { font_id: f_body(14.0 * z), color, background: bg, line_height: Some(20.3 * z), ..Default::default() };
			match side {
				None => job.append(tr("— нет —", "- none -"), 0.0, fmt(Color32::TRANSPARENT, p.muted)),
				Some(s) if pair.service && compare::identifier(&s.text) => {
					job.append(tr("—НЕ ТРОГАТЬ— ", "-DO NOT TOUCH- "), 0.0, egui::TextFormat { font_id: f_semi(13.0 * z), color: p.warn, line_height: Some(20.3 * z), ..Default::default() });
					job.append(s.text.trim(), 0.0, fmt(Color32::TRANSPARENT, p.muted));
				}
				Some(s) => {
					let with_icons = icons_on && crate::kai_buttons::has_codes(&s.text);
					let mut text = if with_icons { compare::clean(&crate::kai_buttons::mark_codes(&s.text), true) } else { compare::clean(&s.text, true) };
					if let Some(c) = rec.and_then(|c| char::from_u32(crate::kai_buttons::SENT0 + c)) {
						text = format!("{c} {text}");
					}
					if let Some(tail) = &rank {
						let t: String = tail.iter().filter_map(|&c| char::from_u32(crate::kai_buttons::SENT0 + c)).collect();
						if !t.is_empty() {
							text = format!("{text} {t}");
						}
					}
					let chars: Vec<char> = text.chars().collect();
					let mut mark = vec![0u8; chars.len()];
					let mut out_n = 0usize;
					for (a, b) in compare::highlights(&text, &self.terms.2) {
						for m in &mut mark[a..b.min(chars.len())] {
							*m = 1;
						}
					}
					for (a, b) in compare::highlights(&text, dict_terms) {
						for m in &mut mark[a..b.min(chars.len())] {
							*m = 2;
						}
					}
					let mut k = 0;
					while k < chars.len() {
						let m = mark[k];
						let mut e = k;
						while e < chars.len() && mark[e] == m {
							e += 1;
						}
						let bg = match m {
							1 => p.hit,
							2 => p.dict,
							_ => Color32::TRANSPARENT,
						};
						let mut a = k;
						while a < e {
							let mut b = a;
							while b < e && crate::kai_buttons::code_of(chars[b]).is_none() {
								b += 1;
							}
							if b > a {
								let seg: String = chars[a..b].iter().collect();
								out_n += b - a;
								job.append(&seg, 0.0, fmt(bg, p.text));
							}
							if b < e {
								let n = crate::kai_buttons::code_of(chars[b]).unwrap_or(0);
								if self.btn.drawable(n) {
									let asp = self.btn.aspect(n).clamp(0.6, 3.5);
									job.append("\u{2003}", 2.0, egui::TextFormat { font_id: f_body(21.0 * asp * z), color: Color32::TRANSPARENT, background: bg, line_height: Some(20.3 * z), ..Default::default() });
									icons.push((out_n, 1, n));
									out_n += 1;
								} else {
									let lab = if compare::ed7().is_some() { format!(" {} ", crate::kai_buttons::code_label(n)) } else { format!(" {n} ") };
									out_n += lab.chars().count();
									job.append(&lab, 1.0, egui::TextFormat { font_id: f_mono(11.5 * z), color: p.muted, background: p.div2, line_height: Some(20.3 * z), ..Default::default() });
								}
								b += 1;
							}
							a = b;
						}
						k = e;
					}
					if chars.is_empty() {
						job.append(" ", 0.0, fmt(Color32::TRANSPARENT, p.text));
					}
				}
			}
			(ui.fonts(|f| f.layout_job(job)), icons)
		};
		let (en, en_icons) = make(&pair.en, &self.terms.0, wen);
		let (ru, ru_icons) = make(&pair.ru, &self.terms.1, wru);
		let ok = if self.mark_ok {
			compare::dict_status(&pair.en.as_ref().map(|s| s.text.clone()).unwrap_or_default(), &pair.ru.as_ref().map(|s| s.text.clone()).unwrap_or_default(), &self.dict)
		} else {
			0
		};
		let mut h = en.size().y.max(ru.size().y).max(34.0) + 18.0;
		if self.row_face(i).is_some() {
			h = h.max(FACE + 12.0);
		}
		if self.row_voice(i).is_some() {
			h = h.max(64.0);
		}
		RowLayout { h, en, ru, ok, ru_icons, en_icons }
	}

	fn face_pack() -> &'static str {
		use crossbell_tool::kai::game::{self, Game};
		match compare::ed7() {
			Some(crossbell_tool::scena::Title::Azure) => "Azure/faces.bin",
			Some(_) => "Zero/faces.bin",
			None => match game::current() {
				Game::Daybreak => "Daybreak-1/faces.bin",
				Game::Daybreak2 => "Daybreak-2/faces.bin",
				Game::Horizon => "Horizon/faces.bin",
			},
		}
	}

	fn row_face(&self, i: usize) -> Option<u32> {
		let d = self.data.as_ref()?;
		let pair = &d.pairs[*self.rows.get(i)?];
		if compare::ed7().is_some() {
			return pair.ru.as_ref().and_then(|s| face_code(&s.text)).or_else(|| pair.en.as_ref().and_then(|s| face_code(&s.text)));
		}
		if let Some(id) = pair.who {
			return compare::speaker_face(Self::face_pack().split('/').next()?, id);
		}
		if !pair.file.ends_with("t_active_voice.xlsx") {
			return None;
		}
		let n: usize = pair.id.strip_prefix("ActiveVoiceTableData#")?.strip_suffix(".voice_subtitle")?.parse().ok()?;
		let av = crate::kai_buttons::pack::get(&Self::face_pack().replace("faces.bin", "active_voice.bin"))?;
		if av.len() < 12 || &av[..5] != b"KAVF1" {
			return None;
		}
		let code = u32::from_le_bytes(av.get(12 + n * 4..16 + n * 4)?.try_into().ok()?);
		(code != u32::MAX).then_some(code)
	}

	fn row_voice(&self, i: usize) -> Option<String> {
		let d = self.data.as_ref()?;
		let pair = &d.pairs[*self.rows.get(i)?];
		if compare::ed7().is_some() {
			let t = pair.en.as_ref().or(pair.ru.as_ref())?;
			return voice_code(&t.text).map(|v| lformat!("голос {v}", "voice {v}"));
		}
		(pair.file.ends_with("t_active_voice.xlsx") && pair.id.ends_with(".voice_subtitle")).then(|| tr("голос", "voice").to_string())
	}

	fn face(&mut self, ctx: &egui::Context, code: u32) -> Option<egui::TextureHandle> {
		let game = Self::face_pack();
		if let Some(t) = self.face_tex.get(&(game, code)) {
			return t.clone();
		}
		let idx = self.face_idx.entry(game).or_insert_with(|| crate::kai_buttons::pack::get(game).map(face_index).unwrap_or_default());
		let tex = idx.get(&code).and_then(|png| image::load_from_memory(png).ok()).map(|img| {
			let img = img.to_rgba8();
			let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
			ctx.load_texture(format!("face{game}_{code}"), ci, egui::TextureOptions::LINEAR)
		});
		self.face_tex.insert((game, code), tex.clone());
		tex
	}

	fn open_inline(&mut self, row: usize, col: usize) {
		let text = if col == 0 { self.row_file(row) } else { self.row_text(row, col) };
		if let Some(t) = text {
			self.inline = Some(Inline { row, col, text: t });
			self.selected = Some(row);
		}
	}

	fn drag_and_drop(&mut self, ctx: &egui::Context) {
		let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
		let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).filter(|p| p.is_dir()).collect());
		let pos = ctx.input(|i| i.pointer.hover_pos());
		if hovering && self.loading.is_none() {
			let p = self.pal;
			let screen = ctx.screen_rect().shrink(8.0);
			let right = pos.map(|pp| pp.x >= self.drop_split).unwrap_or(false);
			egui::Area::new(egui::Id::new("drop")).order(egui::Order::Foreground).fixed_pos(Pos2::ZERO).show(ctx, |ui| {
				ui.painter().rect_filled(screen, 12, p.bg.gamma_multiply(0.86));
				let half = |r: bool| if r { Rect::from_min_max(Pos2::new(self.drop_split.min(screen.max.x), screen.min.y), screen.max) } else { Rect::from_min_max(screen.min, Pos2::new(self.drop_split.min(screen.max.x), screen.max.y)) };
				for r in [false, true] {
					let rr = half(r).shrink(6.0);
					let on = r == right;
					let st = Stroke::new(if on { 2.5 } else { 1.0 }, if on { p.accent } else { p.div });
					ui.painter().extend(egui::Shape::dashed_line(&[rr.left_top(), rr.right_top(), rr.right_bottom(), rr.left_bottom(), rr.left_top()], st, 8.0, 6.0));
					ui.painter().text(rr.center(), Align2::CENTER_CENTER, tr("Яп/Англ/Рус файлы", "JP/EN/RU files"), f_semi(22.0), if on { p.accent } else { p.muted });
				}
			});
		}
		if dropped.is_empty() || self.loading.is_some() {
			return;
		}
		if dropped.len() >= 2 {
			self.en = dropped[0].display().to_string();
			self.ru = dropped[1].display().to_string();
			self.flash(tr("Указаны обе папки — нажмите «Загрузить»", "Both folders are set: click 'Load'"));
		} else if pos.map(|pp| pp.x >= self.drop_split).unwrap_or(!self.en.trim().is_empty()) {
			self.ru = dropped[0].display().to_string();
			self.flash(tr("Правая папка указана", "Right folder set"));
		} else {
			self.en = dropped[0].display().to_string();
			self.flash(tr("Левая папка указана", "Left folder set"));
		}
		self.save_settings();
	}

	fn open_file(&mut self, row: usize, ru: bool) {
		if let Some(&i) = self.rows.get(row) {
			self.open_pair_file(i, ru);
		}
	}

	fn open_pair_file(&mut self, i: usize, ru: bool) {
		let Some(d) = self.data.as_ref() else { return };
		let Some(pair) = d.pairs.get(i) else { return };
		let side = if ru { &pair.ru } else { &pair.en };
		let msg = match side {
			None => tr("Этой строки в файле нет", "This line is not in the file").to_string(),
			Some(s) => match open_at_line(&s.path, s.line) {
				Ok(how) => lformat!("Открыто: {} — {how}", "Opened: {}, {how}", s.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
				Err(e) => e,
			},
		};
		self.flash(&msg);
	}

	fn copy(&mut self, ctx: &egui::Context, row: usize, what: u8) {
		let Some(d) = self.data.as_ref() else { return };
		let Some(&i) = self.rows.get(row) else { return };
		let p = &d.pairs[i];
		let side = |s: &Option<compare::Side>| s.as_ref().map(|s| compare::clean(&s.text, true)).unwrap_or_default();
		let (t, label) = match what {
			1 => (side(&p.en), tr("левый столбец", "left column")),
			2 => (side(&p.ru), tr("правый столбец", "right column")),
			3 => (display_name(p), tr("имя файла", "file name")),
			_ => (format!("{}\n{}", side(&p.en), side(&p.ru)), tr("оба", "both")),
		};
		ctx.copy_text(t);
		self.flash(&lformat!("Скопировано: {label}", "Copied: {label}"));
	}

	fn header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, x0: f32, w: f32, y: f32) {
		let p = self.pal;
		let gap = 14.0;
		let last = 170.0;
		let unit = (w - last - gap * 3.0) / 2.8;
		let (c1, c2, c3) = (unit, unit, unit * 0.8);
		let lh = 16.0;
		let busy = self.loading.is_some();
		let caption = |ui: &egui::Ui, x: f32, w: f32| {
			let long = ui.fonts(|f| f.layout_job(caps_job(tr("Английские/Японские/Русские файлы", "English/Japanese/Russian files"), p.muted)));
			let g = if long.size().x <= w - 8.0 { long } else { ui.fonts(|f| f.layout_job(caps_job(tr("Яп/Англ/Рус файлы", "JP/EN/RU files"), p.muted))) };
			ui.painter().galley(Pos2::new(x, y), g, p.muted);
		};
		caption(ui, x0, c1);
		let fy = y + lh + 5.0;
		{
			let (ch, cl) = path_and_browse(ui, &p, "en", &mut self.en, Rect::from_min_size(Pos2::new(x0, fy), Vec2::new(c1, 36.0)), !busy);
			if cl {
				if let Some(d) = pick_folder(&self.en) {
					self.en = d.display().to_string();
					self.save_settings();
				}
			}
			if ch {
				self.save_settings();
			}
		}
		let x2 = x0 + c1 + gap;
		self.drop_split = x2 - gap / 2.0;
		caption(ui, x2, c2);
		{
			let rect = Rect::from_min_size(Pos2::new(x2, fy), Vec2::new(c2, 36.0));
			let (ch, cl) = path_and_browse(ui, &p, "ru", &mut self.ru, rect, !busy);
			if cl {
				if let Some(d) = pick_folder(&self.ru) {
					self.ru = d.display().to_string();
					self.save_settings();
				}
			}
			if ch {
				self.save_settings();
			}
		}
		let x3 = x2 + c2 + gap;
		ui.painter().galley(Pos2::new(x3, y), ui.fonts(|f| f.layout_job(caps_job(tr("Словарь", "Dictionary"), p.muted))), p.muted);
		{
			let full_r = Rect::from_min_size(Pos2::new(x3, fy), Vec2::new(c3, 36.0));
			let font = f_body(13.0);
			let bw = small_button_width(ui, tr("Выбрать", "Choose"), true);
			let rect = Rect::from_min_max(full_r.min, Pos2::new(full_r.max.x - bw - 8.0, full_r.max.y));
			ui.painter().rect(rect, 8, p.surface, Stroke::new(1.0, p.div), StrokeKind::Inside);
			let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(Pos2::new(rect.max.x + 8.0, rect.min.y), full_r.max)));
			let resp = small_button(&mut bu, &p, tr("Выбрать", "Choose"), Some("book"), 36.0, true, p.text);
			let trect = Rect::from_min_max(Pos2::new(rect.min.x + 11.0, rect.min.y), Pos2::new(rect.max.x - 8.0, rect.max.y));
			let mut job = egui::text::LayoutJob::default();
			if self.dict_path.as_os_str().is_empty() {
				job.append(tr("не выбран", "not chosen"), 0.0, egui::TextFormat { font_id: font.clone(), color: p.muted, ..Default::default() });
			} else if self.dict_err.is_some() {
				job.append(tr("ошибка в словаре", "dictionary error"), 0.0, egui::TextFormat { font_id: font.clone(), color: p.warn, ..Default::default() });
			} else {
				let name = self.dict_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
				job.append(&name, 0.0, egui::TextFormat { font_id: font.clone(), color: p.text, ..Default::default() });
				job.append(&format!("  {}", plural(self.dict.len(), tr("слово", "word"), tr("слова", "words"), tr("слов", "words"))), 0.0, egui::TextFormat { font_id: font.clone(), color: p.muted, ..Default::default() });
			}
			job.wrap = egui::text::TextWrapping { max_width: trect.width(), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
			let g = ui.fonts(|f| f.layout_job(job));
			ui.painter().with_clip_rect(trect).galley(Pos2::new(trect.min.x, trect.center().y - g.size().y / 2.0), g, p.text);
			let resp = resp.on_hover_text(if let Some(e) = &self.dict_err { e.clone() } else { tr("Словарь: .dat, .json или .txt — подсветка и отметка строк «как в словаре»", "Dictionary: .dat, .json or .txt, used for highlights and for marking lines 'as in the dictionary'").into() });
			if resp.clicked() {
				let start = self.dict_path.parent().map(|d| d.display().to_string()).unwrap_or_default();
				if let Some(f) = pick_files(&start, tr("Выберите словарь", "Choose a dictionary"), tr("Словарь", "Dictionary"), &["dat", "json", "txt"], false).into_iter().next() {
					self.load_dict(&f, true);
					self.save_settings();
					self.reset_layouts();
				}
			}
		}
		let x4 = x3 + c3 + gap;
		let prog = self.loading.as_ref().map(|l| {
			let t = l.total.load(Ordering::SeqCst);
			if t == 0 { 0.0 } else { l.done.load(Ordering::SeqCst) as f32 / t as f32 }
		});
		let status = match prog {
			Some(pr) => lformat!("Загрузка… {}%", "Loading… {}%", (pr * 100.0).round() as i32),
			None => self.load_status.clone(),
		};
		let mut job = egui::text::LayoutJob::default();
		if self.load_warn > 0 && prog.is_none() {
			job.append(&lformat!("Не прочитано: {}", "Not read: {}", plural(self.load_warn, tr("файл", "file"), tr("файла", "files"), tr("файлов", "files"))), 0.0, egui::TextFormat { font_id: f_body(11.0), color: p.warn, ..Default::default() });
		} else {
			job.append(&status, 0.0, egui::TextFormat { font_id: f_body(11.0), color: p.muted, ..Default::default() });
		}
		job.wrap = egui::text::TextWrapping { max_width: last, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
		let g = ui.fonts(|f| f.layout_job(job));
		let sresp = ui.interact(Rect::from_min_size(Pos2::new(x4, y), Vec2::new(last, lh)), ui.id().with("loadstat"), Sense::hover());
		ui.painter().galley(Pos2::new(x4 + (last - g.size().x) / 2.0, y + 2.0), g, p.muted);
		if self.load_warn > 0 {
			if let Some(d) = &self.data {
				sresp.on_hover_text(lformat!("{}\n\nНе прочитаны (ошибка в файле — например, «\\» без n):\n{}", "{}\n\nNot read (an error in the file, for example a '\\' without n):\n{}", self.load_status, d.warnings.iter().take(10).cloned().collect::<Vec<_>>().join("\n")));
			}
		}
		{
			let rect = Rect::from_min_size(Pos2::new(x4, fy), Vec2::new(last, 36.0));
			let mut r = ui.new_child(egui::UiBuilder::new().max_rect(rect));
			if primary_button_icon(&mut r, &p, if busy { tr("Загрузка…", "Loading…") } else { tr("Загрузить", "Load") }, Some("download"), Vec2::new(last, 36.0), 8, f_semi(17.0), !busy).clicked() {
				self.start_load(ctx);
			}
			let pr = prog.unwrap_or(if self.data.is_some() { 1.0 } else { 0.0 });
			if pr > 0.0 {
				let bar = Rect::from_min_size(Pos2::new(rect.min.x, rect.max.y - 3.0), Vec2::new(rect.width() * pr.clamp(0.0, 1.0), 3.0));
				ui.painter().with_clip_rect(rect).rect_filled(bar, egui::CornerRadius { nw: 0, ne: 0, sw: 8, se: if pr > 0.98 { 8 } else { 0 } }, p.ok_strong);
			}
		}
	}

	fn top_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, full: Rect) -> f32 {
		let p = self.pal;
		let h = 52.0;
		let bar = Rect::from_min_size(full.min, Vec2::new(full.width(), h));
		ui.painter().rect_filled(bar, 0, p.surface);
		ui.painter().line_segment([Pos2::new(bar.min.x, bar.max.y - 0.5), Pos2::new(bar.max.x, bar.max.y - 0.5)], Stroke::new(1.0, p.div));
		let ed7 = compare::ed7().is_some();
		let (name, ver, games) = ("CrossbellCompare", env!("VER_CROSSBELLCOMPARE"), "Zero · Azure");
		let ver = format!("{ver}{}", env!("VER_SUFFIX"));
		let cy = bar.center().y;
		let mut x = bar.min.x + 22.0;
		let logo = Rect::from_min_size(Pos2::new(x, cy - 15.0), Vec2::splat(30.0));
		ui.painter().rect_filled(logo, 7, p.accent);
		paint_icon(ui, "columns", logo.shrink(6.0), p.on_accent);
		x = logo.max.x + 12.0;
		let g = ui.painter().layout_no_wrap(name.to_string(), f_semi(15.0), p.text);
		let gw = g.size().x;
		ui.painter().galley(Pos2::new(x, cy - g.size().y / 2.0), g, p.text);
		x += gw + 10.0;
		let g = ui.painter().layout_no_wrap(ver.trim().to_string(), f_body(11.0), p.muted);
		let vr = Rect::from_min_size(Pos2::new(x, cy - 10.0), Vec2::new(g.size().x + 14.0, 20.0));
		ui.painter().rect_filled(vr, 5, p.div2);
		ui.painter().galley(Pos2::new(vr.min.x + 7.0, cy - g.size().y / 2.0), g, p.muted);
		x = vr.max.x + 12.0;
		let right = Rect::from_min_max(Pos2::new(x, bar.min.y), Pos2::new(bar.max.x - 22.0, bar.max.y));
		let mut r = ui.new_child(egui::UiBuilder::new().max_rect(right).layout(egui::Layout::right_to_left(egui::Align::Center)));
		r.spacing_mut().item_spacing.x = 8.0;
		if small_button(&mut r, &p, tr("О программе", "About"), Some("info"), 30.0, true, p.text).clicked() {
			let st = if ed7 { "KisekiTool_compare.ini" } else { "KaiCompare.ini" };
			self.dialog = Dialog::Info(lformat!("{name} {ver}\nДата сборки: {}\nИгры: {}\n\nНастройки: {}\nПапка программы: {}", "{name} {ver}\nBuild date: {}\nGames: {}\n\nSettings: {}\nProgram folder: {}",
				build_date(),
				games.replace(" · ", ", "),
				exe_dir().join(st).display(),
				exe_dir().display()
			));
		}
		if small_button(&mut r, &p, tr("Подсказки", "Tips"), Some("help"), 30.0, true, p.text).clicked() {
			self.dialog = Dialog::Info(
				tr("Ctrl+F — к полю поиска · Enter — найти\nF5 — загрузить заново · Shift+F5 — без кэша\nCtrl+1 / Ctrl+2 — «Сверка» / «Проверки»\nCtrl+колёсико, Ctrl+плюс/минус, Ctrl+0 — масштаб текста таблицы\nF11 — во весь экран\n\nДвойной клик по ячейке — выделить часть текста\nПравый клик — копировать, открыть файл на строке\nCtrl+C — копировать столбец, по которому щёлкнули\nПапки можно перетащить в окно мышью", "Ctrl+F: go to the search field · Enter: search\nF5: reload · Shift+F5: reload without the cache\nCtrl+1 / Ctrl+2: Compare / Checks\nCtrl+wheel, Ctrl+plus/minus, Ctrl+0: table text size\nF11: full screen\n\nDouble-click a cell to select part of its text\nRight-click: copy, open the file at this line\nCtrl+C: copy the clicked column\nFolders can be dragged onto the window").into(),
			);
		}
		if small_button(&mut r, &p, if ui_en() { "RU" } else { "EN" }, Some("languages"), 30.0, true, p.text).on_hover_text(tr("Switch to English", "Переключить на русский")).clicked() {
			UI_EN.store(!ui_en(), std::sync::atomic::Ordering::Relaxed);
			self.save_settings();
			self.reset_layouts();
		}
		if theme_button(&mut r, &p, 30.0).clicked() {
			self.pal = Pal::get(!self.pal.dark);
			apply_visuals(ctx, &self.pal);
			self.save_settings();
			self.reset_layouts();
		}
		let space = r.available_width();
		let g = ui.painter().layout_no_wrap(games.to_string(), f_body(12.0), p.muted);
		if g.size().x + 12.0 < space {
			ui.painter().galley(Pos2::new(x, cy - g.size().y / 2.0), g, p.muted);
		}
		h
	}

	fn search_row(&mut self, ui: &mut egui::Ui, x0: f32, w: f32, y: f32) {
		let p = self.pal;
		let h = 42.0;
		let fbtn = 26.0 * 2.0 + ui.fonts(|f| f.layout_no_wrap(tr("Найти", "Search").into(), f_semi(17.0), p.text).size().x);
		let sw = w - fbtn - 12.0;
		let srect = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(sw, h));
		ui.painter().rect(srect, 10, p.surface, Stroke::new(1.0, p.accent), StrokeKind::Inside);
		paint_icon(ui, "search", Rect::from_min_size(Pos2::new(srect.min.x + 14.0, srect.center().y - 9.0), Vec2::splat(18.0)), p.accent);
		let trect = Rect::from_min_max(Pos2::new(srect.min.x + 14.0 + 18.0 + 10.0, srect.min.y), Pos2::new(srect.max.x - 14.0, srect.max.y));
		let mut c = ui.new_child(egui::UiBuilder::new().max_rect(trect).layout(egui::Layout::left_to_right(egui::Align::Center)));
		let te = egui::TextEdit::singleline(&mut self.search)
			.id(egui::Id::new("search_field"))
			.frame(false)
			.font(f_body(16.0))
			.hint_text(egui::RichText::new(tr("Слово на Яп, Англ, Рус", "A word in JP, EN or RU")).color(p.muted))
			.desired_width(trect.width())
			.margin(egui::Margin::ZERO);
		let resp = c.add(te);
		let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
		let mut r = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::new(srect.max.x + 12.0, y), Vec2::new(fbtn, h))));
		let find = primary_button_icon(&mut r, &p, tr("Найти", "Search"), Some("search"), Vec2::new(fbtn, h), 10, f_semi(17.0), true).clicked();
		if enter || find {
			self.save_settings();
			self.do_search();
			if enter {
				resp.request_focus();
			}
		}
	}

	fn options_row(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, x0: f32, w: f32, y: f32) {
		let p = self.pal;
		let row = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 30.0));
		let mut r = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
		r.spacing_mut().item_spacing.x = 18.0;
		let mut whole = self.whole;
		if checkbox(&mut r, &p, &mut whole, tr("Целые слова", "Whole words"), 16.0, f_body(13.0)).changed() {
			self.whole = whole;
			self.save_settings();
			if self.file_sel > 0 {
				self.reset_layouts();
			} else if self.searched && !self.query.is_empty() {
				self.do_search();
			} else {
				self.reset_layouts();
			}
		}
		let mut case = self.case;
		if checkbox(&mut r, &p, &mut case, tr("Учитывать регистр", "Match case"), 16.0, f_body(13.0)).changed() {
			self.case = case;
			self.save_settings();
			if self.searched && !self.query.is_empty() && self.file_sel == 0 {
				self.do_search();
			}
		}
		let mut ok = self.mark_ok;
		if checkbox(&mut r, &p, &mut ok, tr("Отмечать «как в словаре»", "Mark 'as in the dictionary'"), 16.0, f_body(13.0)).on_hover_text(tr("Строка зелёная, если в переводе стоят слова из словаря", "A line is green if the translation uses the dictionary's words")).changed() {
			self.mark_ok = ok;
			self.save_settings();
			self.reset_layouts();
		}
		let left_used = r.min_rect().max.x - x0 + 18.0;
		let mut r = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::right_to_left(egui::Align::Center)));
		r.spacing_mut().item_spacing.x = 10.0;
		let _ = ctx;
		let bl = if self.btn.on { lformat!("Значки: {}", "Icons: {}", crate::kai_buttons::devices()[self.btn.device]) } else { tr("Значки кнопок: выкл", "Button icons: off").to_string() };
		if small_button(&mut r, &p, &bl, Some("gamepad"), 30.0, true, if self.btn.on { p.accent_strong } else { p.text }).on_hover_text(tr("Значки предметов и кнопок вместо кодов <I…> в русском тексте", "Item and button icons instead of <I…> codes in the Russian text")).clicked() {
			self.btn_open = true;
		}
		r.add_space(12.0);
		let cw = (w * 0.22).clamp(200.0, 380.0).min((r.available_width() - left_used - 12.0).max(140.0));
		self.file_items[0] = tr("(все найденные — через поиск)", "(all found, use search)").to_string();
		let old = self.file_sel;
		if combo(&mut r, &p, "file", &self.file_items.clone(), &mut self.file_sel, cw, 30.0, 7, f_body(13.0)) && old != self.file_sel {
			if self.file_sel == 0 {
				if !self.query.is_empty() {
					self.do_search();
				} else {
					self.rows.clear();
					self.searched = false;
					self.reset_layouts();
				}
			} else {
				self.show_file();
			}
		}
		let lw = r.fonts(|f| f.layout_no_wrap(tr("Файл целиком", "Whole file").into(), f_body(13.0), p.muted).size().x);
		if r.available_width() - left_used >= lw + 6.0 {
			r.label(egui::RichText::new(tr("Файл целиком", "Whole file")).font(f_body(13.0)).color(p.muted));
		}
	}

	fn result_row(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, x0: f32, w: f32, y: f32) {
		let p = self.pal;
		let mut x = x0;
		let cy = y + 13.0;
		if self.searched {
			let g = ui.painter().layout_no_wrap(self.found.clone(), f_semi(19.0), p.text);
			let gw = g.size().x;
			ui.painter().galley(Pos2::new(x, cy - g.size().y / 2.0), g, p.text);
			x += gw + 14.0;
			if let (Some(chip), true) = (&self.chip, self.file_sel == 0) {
				let max = (x0 + w - x) * 0.55;
				let mut job = egui::text::LayoutJob::single_section(chip.clone(), egui::TextFormat { font_id: f_body(12.0), color: p.text, ..Default::default() });
				job.wrap = egui::text::TextWrapping { max_width: max, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
				let g = ui.fonts(|f| f.layout_job(job));
				let cr = Rect::from_min_size(Pos2::new(x, cy - 11.0), Vec2::new(g.size().x + 20.0, 22.0));
				ui.painter().rect_filled(cr, 11, p.dict);
				ui.painter().galley(Pos2::new(cr.min.x + 10.0, cr.center().y - g.size().y / 2.0), g, p.text);
				x = cr.max.x + 14.0;
			}
		}
		let _ = (ctx, x);
	}

	fn auto_scroll(&mut self, ctx: &egui::Context, body: Rect) -> f32 {
		let (mid, pos, any_other) = ctx.input(|i| {
			(
				i.pointer.button_pressed(egui::PointerButton::Middle),
				i.pointer.hover_pos(),
				i.pointer.button_pressed(egui::PointerButton::Primary) || i.pointer.button_pressed(egui::PointerButton::Secondary) || i.key_pressed(egui::Key::Escape),
			)
		});
		let mid_up = ctx.input(|i| i.pointer.button_released(egui::PointerButton::Middle));
		if mid {
			if self.auto_anchor.is_some() {
				self.auto_anchor = None;
			} else if let Some(pp) = pos.filter(|pp| body.contains(*pp)) {
				self.auto_anchor = Some(pp);
				self.auto_since = Some(Instant::now());
			}
		} else if any_other {
			self.auto_anchor = None;
		} else if mid_up {
			if let (Some(a), Some(t)) = (self.auto_anchor, self.auto_since) {
				let moved = pos.map(|pp| (pp.y - a.y).abs() > 12.0).unwrap_or(false);
				if moved || t.elapsed().as_secs_f32() > 0.4 {
					self.auto_anchor = None;
				}
			}
		}
		if self.auto_anchor.is_none() {
			self.auto_since = None;
		}
		match (self.auto_anchor, pos) {
			(Some(a), Some(pp)) => {
				let dy = pp.y - a.y;
				let dead = 10.0;
				let dt = ctx.input(|i| i.stable_dt).min(0.05);
				if dy.abs() > dead { (dy - dead * dy.signum()) * 6.0 * dt } else { 0.0 }
			}
			_ => 0.0,
		}
	}

	fn auto_cursor(&self, ctx: &egui::Context, v: f32) {
		if self.auto_anchor.is_some() {
			ctx.set_cursor_icon(if v > 0.0 { egui::CursorIcon::ResizeSouth } else if v < 0.0 { egui::CursorIcon::ResizeNorth } else { egui::CursorIcon::ResizeVertical });
			ctx.request_repaint();
		}
	}

	fn table(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: Rect) {
		let p = self.pal;
		ui.painter().rect(rect, 12, Color32::TRANSPARENT, Stroke::new(1.0, p.div), StrokeKind::Inside);
		let inner = rect.shrink(1.0);
		let head_h = 34.0;
		let wfile = clamp_safe(self.col_file, 90.0, inner.width() * 0.4);
		let rest = inner.width() - wfile;
		let wen = clamp_safe(rest * self.col_split, 120.0, rest - 120.0);
		let wru = rest - wen;
		let xs = [inner.min.x, inner.min.x + wfile, inner.min.x + wfile + wen, inner.max.x];
		for (i, t) in [tr("Файл", "File"), tr("Яп, Англ, Рус", "JP, EN, RU"), tr("Яп, Англ, Рус", "JP, EN, RU")].iter().enumerate() {
			ui.painter().galley(Pos2::new(xs[i] + 12.0, inner.min.y + 11.0), ui.fonts(|f| f.layout_job(caps_job(t, p.muted))), p.muted);
		}
		let hy = inner.min.y + head_h;
		ui.painter().line_segment([Pos2::new(inner.min.x, hy), Pos2::new(inner.max.x, hy)], Stroke::new(1.0, p.div));
		for k in 1..=2 {
			let x = xs[k];
			let hr = Rect::from_min_max(Pos2::new(x - 4.0, inner.min.y), Pos2::new(x + 4.0, inner.max.y));
			let resp = ui.interact(hr, ui.id().with(("colsep", k)), Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
			if resp.dragged() {
				let dx = resp.drag_delta().x;
				if k == 1 {
					self.col_file = clamp_safe(self.col_file + dx, 90.0, inner.width() * 0.4);
				} else {
					if rest > 0.0 {
						self.col_split = ((wen + dx) / rest).clamp(0.15, 0.85);
					}
				}
			}
		}
		let body = Rect::from_min_max(Pos2::new(inner.min.x, hy + 1.0), inner.max);
		let key = ((wen.round()) as u32, (wru.round()) as u32, p.dark, (self.zoom * 100.0).round() as u32);
		if key != self.layout_key {
			self.layout_key = key;
			for l in &mut self.layouts {
				*l = None;
			}
			self.layout_next = 0;
		}
		let tw_en = wen - 24.0;
		let tw_ru = wru - 24.0;
		let n = self.rows.len();
		if self.data.is_none() || n == 0 {
			for k in 1..=2 {
				ui.painter().line_segment([Pos2::new(xs[k], inner.min.y), Pos2::new(xs[k], inner.max.y)], Stroke::new(1.0, p.text.gamma_multiply(0.2)));
			}
			let msg = if self.loading.is_some() && self.search_pending {
				tr("Идёт загрузка — поиск выполнится, когда файлы загрузятся", "Loading: the search will run once the files are loaded")
			} else if self.loading.is_some() {
				tr("Загрузка…", "Loading…")
			} else if self.data.is_none() {
				tr("Укажите папки и нажмите «Загрузить»", "Choose the folders and click 'Load'")
			} else if self.searched {
				tr("Ничего не найдено", "Nothing found")
			} else {
				tr("Впишите слово и нажмите Enter", "Type a word and press Enter")
			};
			ui.painter().text(body.center(), Align2::CENTER_CENTER, msg, f_body(14.0), p.muted);
			return;
		}
		let budget = Instant::now();
		while self.layout_next < n && budget.elapsed().as_millis() < 25 {
			if self.layouts[self.layout_next].is_none() {
				let l = self.layout_row(ui, self.layout_next, tw_en, tw_ru);
				self.layouts[self.layout_next] = Some(l);
			}
			self.layout_next += 1;
		}
		if self.layout_next < n {
			ctx.request_repaint();
		}
		let est = 38.0;
		let auto_v = self.auto_scroll(ctx, body);
		let rows_rect = Rect::from_min_max(body.min, Pos2::new(body.max.x - BAR_W, body.max.y));
		let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rows_rect));
		child.set_clip_rect(rows_rect.intersect(ui.clip_rect()));
		let mut sa = egui::ScrollArea::vertical().id_salt("table").auto_shrink([false, false]).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
		if let Some(o) = self.bar_set.remove("table") {
			sa = sa.vertical_scroll_offset(o);
		}
		if let Some(r) = self.scroll_to.take() {
			for k in 0..=r.min(n - 1) {
				if self.layouts[k].is_none() {
					let l = self.layout_row(ui, k, tw_en, tw_ru);
					self.layouts[k] = Some(l);
				}
			}
			let off: f32 = self.layouts[..r.min(n)].iter().map(|l| l.as_ref().map(|l| l.h).unwrap_or(est)).sum();
			sa = sa.vertical_scroll_offset((off - 60.0).max(0.0));
			self.scroll_top = false;
		} else if self.scroll_top {
			sa = sa.vertical_scroll_offset(0.0);
			self.scroll_top = false;
		}
		let heights: Vec<f32> = self.layouts.iter().map(|l| l.as_ref().map(|l| l.h).unwrap_or(est)).collect();
		let total_h: f32 = heights.iter().sum();
		let mut clicked: Option<(usize, usize, bool, bool)> = None;
		let mut hover_pos = None;
		let out = sa.show_viewport(&mut child, |ui, vp| {
			ui.set_min_height(total_h);
			ui.set_width(rows_rect.width());
			let origin = ui.min_rect().min;
			let mut y = 0.0;
			let mut first = true;
			for i in 0..n {
				let h = heights[i];
				if y + h < vp.min.y {
					y += h;
					continue;
				}
				if y > vp.max.y {
					break;
				}
				if first {
					self.top_row = i;
					first = false;
				}
				if self.layouts[i].is_none() {
					let l = self.layout_row(ui, i, tw_en, tw_ru);
					self.layouts[i] = Some(l);
				}
				let face = self.row_face(i).and_then(|c| self.face(ctx, c));
				let lay = self.layouts[i].as_ref().unwrap();
				let rr =Rect::from_min_size(Pos2::new(origin.x, origin.y + y), Vec2::new(rows_rect.width(), lay.h));
				let resp = ui.interact(rr, ui.id().with(("row", i)), Sense::click());
				let sel = self.selected == Some(i);
				if sel {
					ui.painter().rect_filled(rr, 0, p.sel);
				} else if lay.ok == 1 {
					ui.painter().rect_filled(rr, 0, p.ok);
				}
				if resp.hovered() {
					ui.painter().rect_filled(Rect::from_min_size(rr.min, Vec2::new(3.0, rr.height())), 0, p.accent);
					hover_pos = Some(i);
				}
				ui.painter().line_segment([Pos2::new(rr.min.x, rr.max.y), Pos2::new(rr.max.x, rr.max.y)], Stroke::new(1.0, p.text.gamma_multiply(0.2)));
				let d = self.data.as_ref().unwrap();
				let pair = &d.pairs[self.rows[i]];
				let side = pair.ru.as_ref().or(pair.en.as_ref());
				let line = side.map(|s| s.line).unwrap_or(0);
				let fx = rr.min.x + 12.0;
				let face_w = if face.is_some() && wfile > FACE + 60.0 { FACE + 8.0 } else { 0.0 };
				if let (Some(t), true) = (&face, face_w > 0.0) {
					let r = Rect::from_min_size(Pos2::new(rr.min.x + wfile - 10.0 - FACE, rr.min.y + 6.0), Vec2::splat(FACE));
					ui.painter().image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
				}
				let fr = Rect::from_min_size(Pos2::new(fx, rr.min.y + 9.0), Vec2::new(wfile - 24.0 - face_w, 16.0));
				paint_text_clipped(ui, fr, &display_name(pair), f_mono(11.5), p.muted, Align2::LEFT_TOP);
				let where_ = if line > 0 { lformat!("стр. {line}", "line {line}") } else { pair.id.clone() };
				ui.painter().text(Pos2::new(fx, rr.min.y + 26.0), Align2::LEFT_TOP, where_, f_mono(11.5), p.muted);
				if let Some(v) = self.row_voice(i) {
					let vr = Rect::from_min_size(Pos2::new(fx, rr.min.y + 43.0), Vec2::new(wfile - 24.0 - face_w, 16.0));
					let vresp = ui.interact(vr, ui.id().with(("voice", i)), Sense::hover());
					paint_text_clipped(ui, vr, &format!("♪ {v}"), f_body(11.5), p.accent, Align2::LEFT_TOP);
					vresp.on_hover_text(tr("Озвученная реплика: в игре звучит голос (номер файла озвучки)", "Voiced line: the game plays a voice (voice file number)"));
				}
				let ex = rr.min.x + wfile + 12.0;
				let rx = rr.min.x + wfile + wen + 12.0;
				let en_clip = Rect::from_min_size(Pos2::new(ex - 2.0, rr.min.y), Vec2::new(wen - 20.0, rr.height())).intersect(ui.clip_rect());
				ui.painter().with_clip_rect(en_clip).galley(Pos2::new(ex, rr.min.y + 9.0), lay.en.clone(), p.text);
				let ru_clip = Rect::from_min_size(Pos2::new(rx - 2.0, rr.min.y), Vec2::new(wru - 20.0, rr.height())).intersect(ui.clip_rect());
				ui.painter().with_clip_rect(ru_clip).galley(Pos2::new(rx, rr.min.y + 9.0), lay.ru.clone(), p.text);
				for (g, list, origin_ru, ru_clip) in [(lay.en.clone(), lay.en_icons.clone(), Pos2::new(ex, rr.min.y + 9.0), en_clip), (lay.ru.clone(), lay.ru_icons.clone(), Pos2::new(rx, rr.min.y + 9.0), ru_clip)] {
					for (ci, sl, n) in list {
						let a = g.pos_from_ccursor(egui::text::CCursor::new(ci));
						let b = g.pos_from_ccursor(egui::text::CCursor::new(ci + sl));
						let w = if (b.min.y - a.min.y).abs() < 1.0 && b.min.x > a.min.x { b.min.x - a.min.x } else { 21.0 * self.zoom * self.btn.aspect(n) * sl as f32 };
						let cell = Rect::from_min_size(origin_ru + a.min.to_vec2(), Vec2::new(w, a.height())).shrink2(Vec2::new(1.0, 0.0));
						if let Some((tex, asp)) = self.btn.icon(ctx, n) {
							let h = (20.0 * self.zoom).min(cell.width() / asp).max(4.0);
							let r = Rect::from_center_size(cell.center(), Vec2::new(h * asp, h));
							ui.painter().with_clip_rect(ru_clip).image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
						}
					}
				}
				let col_of = |x: f32| if x < rr.min.x + wfile { 0 } else if x < rr.min.x + wfile + wen { 1 } else { 2 };
				if let Some(pos) = resp.interact_pointer_pos() {
					if resp.double_clicked() {
						clicked = Some((i, col_of(pos.x), true, false));
					} else if resp.clicked() {
						clicked = Some((i, col_of(pos.x), false, false));
					} else if resp.secondary_clicked() {
						clicked = Some((i, col_of(pos.x), false, true));
					}
				}
				y += h;
			}
		});
		let _ = hover_pos;
		for k in 1..=2 {
			ui.painter().line_segment([Pos2::new(xs[k], inner.min.y), Pos2::new(xs[k], inner.max.y)], Stroke::new(1.0, p.text.gamma_multiply(0.2)));
		}
		if let Some((i, col, dbl, right)) = clicked {
			self.selected = Some(i);
			self.click_col = col;
			if dbl {
				self.open_inline(i, col);
			} else if right {
				let pos = ctx.input(|inp| inp.pointer.interact_pos()).unwrap_or_default();
				self.menu = Some((i, pos));
				self.inline = None;
			} else if self.inline.as_ref().map(|t| t.row != i || t.col != col).unwrap_or(false) {
				self.inline = None;
			}
		}
		if auto_v != 0.0 {
			let max_off = (out.content_size.y - out.inner_rect.height()).max(0.0);
			self.bar_set.insert("table", (out.state.offset.y + auto_v).clamp(0.0, max_off));
		}
		self.auto_cursor(ctx, auto_v);
		if let Some(o) = vbar(ui, &p, "table", Rect::from_min_max(Pos2::new(body.max.x - BAR_W + 2.0, body.min.y + 3.0), Pos2::new(body.max.x - 3.0, body.max.y - 3.0)), out.state.offset.y, out.content_size.y, out.inner_rect.height()) {
			self.bar_set.insert("table", o);
			self.auto_anchor = None;
			ctx.request_repaint();
		}
		if let Some(il) = self.inline.clone() {
			let mut y = body.min.y;
			let scroll = out.state.offset.y;
			for h in heights.iter().take(il.row) {
				y += h;
			}
			y -= scroll;
			let (cx, cw) = match il.col { 0 => (xs[0], wfile), 1 => (xs[1], wen), _ => (xs[2], wru) };
			let h = heights.get(il.row).copied().unwrap_or(est);
			let cell = Rect::from_min_size(Pos2::new(cx, y), Vec2::new(cw, h)).shrink(3.0);
			let fits = h > body.height() || (cell.min.y >= body.min.y - 1.0 && cell.max.y <= body.max.y + 1.0);
			if !fits {
				self.inline = None;
				return;
			}
			let mut close = false;
			let area = egui::Area::new(egui::Id::new("inline_cell")).order(egui::Order::Foreground).fixed_pos(cell.min).show(ctx, |ui| {
				egui::Frame::new()
					.fill(p.bg)
					.stroke(Stroke::new(1.0, p.accent))
					.corner_radius(8)
					.inner_margin(egui::Margin::symmetric(8, 6))
					.shadow(egui::epaint::Shadow { offset: [0, 3], blur: 10, spread: 0, color: Color32::from_black_alpha(46) })
					.show(ui, |ui| {
						ui.set_width(cell.width() - 16.0);
						let mut t: &str = &il.text;
						let resp = ui.add(egui::TextEdit::multiline(&mut t).id(egui::Id::new("inline_te")).frame(false).font(if il.col == 0 { f_mono(12.5) } else { f_body(14.0 * self.zoom) }).desired_width(cell.width() - 16.0).desired_rows(1).margin(egui::Margin::ZERO));
						if !resp.has_focus() && ui.input(|i| i.pointer.any_click()) == false {
							resp.request_focus();
						}
					});
			});
			let pos = ctx.input(|i| i.pointer.interact_pos());
			if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter)) {
				close = true;
			}
			if ctx.input(|i| i.pointer.any_pressed()) {
				if let Some(pp) = pos {
					if !area.response.rect.contains(pp) && clicked.map(|c| !c.2).unwrap_or(true) {
						close = true;
					}
				}
			}
			if close {
				self.inline = None;
			}
		}
	}

	fn context_menu(&mut self, ctx: &egui::Context) {
		if let Some((pair, pos)) = self.check_menu {
			match Self::menu_popup(&self.pal, ctx, "check_menu", pos, &[(tr("Открыть в сверке", "Open in Compare"), Some(0)), ("", None), (tr("Открыть файл слева", "Open the left file"), Some(1)), (tr("Открыть файл справа", "Open the right file"), Some(2))]) {
				Some(Some(0)) => {
					self.open_pair(pair);
					self.check_menu = None;
				}
				Some(Some(w)) => {
					self.open_pair_file(pair, w == 2);
					self.check_menu = None;
				}
				Some(None) => self.check_menu = None,
				None => {}
			}
		}
		let Some((row, _)) = self.menu else { return };
		let pos = self.menu.map(|m| m.1).unwrap_or_default();
		let items: [(&str, Option<u8>); 8] = [(tr("Копировать левый столбец", "Copy the left column"), Some(1)), (tr("Копировать правый столбец", "Copy the right column"), Some(2)), (tr("Копировать оба", "Copy both"), Some(0)), ("", None), (tr("Копировать имя файла", "Copy the file name"), Some(3)), ("", None), (tr("Открыть файл слева", "Open the left file"), Some(4)), (tr("Открыть файл справа", "Open the right file"), Some(5))];
		match Self::menu_popup(&self.pal, ctx, "row_menu", pos, &items) {
			Some(Some(w)) => {
				if w >= 4 {
					self.open_file(row, w == 5);
				} else {
					self.copy(ctx, row, w);
				}
				self.menu = None;
			}
			Some(None) => self.menu = None,
			None => {}
		}
	}

	fn menu_popup(p: &Pal, ctx: &egui::Context, id: &str, pos: Pos2, items: &[(&str, Option<u8>)]) -> Option<Option<u8>> {
		if ctx.input(|i| i.smooth_scroll_delta.y != 0.0 || i.raw_scroll_delta.y != 0.0) {
			return Some(None);
		}
		let p = *p;
		let mut choice: Option<u8> = None;
		let area = egui::Area::new(egui::Id::new(id)).order(egui::Order::Foreground).fixed_pos(pos).constrain(true).show(ctx, |ui| {
			egui::Frame::new()
				.fill(p.bg)
				.stroke(Stroke::new(1.0, p.div))
				.corner_radius(10)
				.inner_margin(egui::Margin::same(5))
				.shadow(egui::epaint::Shadow { offset: [0, 12], blur: 32, spread: 0, color: Color32::from_black_alpha(64) })
				.show(ui, |ui| {
					ui.set_min_width(250.0);
					for &(t, what) in items {
						match what {
							None => {
								let (r, _) = ui.allocate_exact_size(Vec2::new(250.0, 9.0), Sense::hover());
								ui.painter().line_segment([Pos2::new(r.min.x + 6.0, r.center().y), Pos2::new(r.max.x - 6.0, r.center().y)], Stroke::new(1.0, p.div));
							}
							Some(w) => {
								let (r, resp) = ui.allocate_exact_size(Vec2::new(250.0, 30.0), Sense::click());
								if resp.hovered() {
									ui.painter().rect_filled(r, 7, p.sel);
								}
								ui.painter().text(Pos2::new(r.min.x + 14.0, r.center().y), Align2::LEFT_CENTER, t, f_body(13.0), p.text);
								if resp.clicked() {
									choice = Some(w);
								}
							}
						}
					}
				});
		});
		if choice.is_some() {
			return Some(choice);
		}
		if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
			return Some(None);
		}
		if ctx.input(|i| i.pointer.any_pressed()) {
			let pp = ctx.input(|i| i.pointer.interact_pos()).unwrap_or_default();
			if !area.response.rect.contains(pp) {
				return Some(None);
			}
		}
		None
	}
}

impl eframe::App for App {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		self.show(ctx);
	}

	fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
		self.save_settings();
	}
}

impl App {
	pub fn on_exit_save(&mut self) {
		self.save_settings();
	}

	pub fn show(&mut self, ctx: &egui::Context) {
		self.poll_load();
		let p = self.pal;
		if ctx.input(|i| i.key_pressed(egui::Key::F11)) || (self.fullscreen && ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.inline.is_none() && self.menu.is_none()) {
			self.fullscreen = !self.fullscreen;
			ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
		}
		let copy_req = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
		let editing = ctx.memory(|m| m.focused().is_some());
		if copy_req && !editing && self.inline.is_none() {
			if let Some(r) = self.selected {
				let what = match self.click_col { 0 => 3, 1 => 1, _ => 2 };
				self.copy(ctx, r, what);
			}
		}
		if matches!(self.dialog, Dialog::None) && self.inline.is_none() && self.menu.is_none() && !self.hidden_open && !self.btn_open {
			let (f5, find, tab1, tab2) = ctx.input_mut(|i| {
				(
					i.key_pressed(egui::Key::F5),
					i.consume_key(egui::Modifiers::COMMAND, egui::Key::F),
					i.consume_key(egui::Modifiers::COMMAND, egui::Key::Num1),
					i.consume_key(egui::Modifiers::COMMAND, egui::Key::Num2),
				)
			});
			if f5 && self.loading.is_none() && self.en_dir().is_dir() && self.ru_dir().is_dir() {
				self.start_load(ctx);
			}
			if find {
				self.page = 0;
				ctx.memory_mut(|m| m.request_focus(egui::Id::new("search_field")));
			}
			if tab1 {
				self.page = 0;
			}
			if tab2 {
				self.page = 1;
			}
			let (zin, zout, zreset, zd) = ctx.input_mut(|i| {
				(
					i.consume_key(egui::Modifiers::COMMAND, egui::Key::Plus) || i.consume_key(egui::Modifiers::COMMAND, egui::Key::Equals),
					i.consume_key(egui::Modifiers::COMMAND, egui::Key::Minus),
					i.consume_key(egui::Modifiers::COMMAND, egui::Key::Num0),
					i.zoom_delta(),
				)
			});
			if self.page == 0 {
				let mut z = self.zoom * zd;
				if zin {
					z += 0.1;
				}
				if zout {
					z -= 0.1;
				}
				if zreset {
					z = 1.0;
				}
				let z = z.clamp(0.7, 2.0);
				if (z - self.zoom).abs() > 0.001 {
					self.zoom = z;
					self.flash(&lformat!("Масштаб текста: {:.0}%", "Text size: {:.0}%", z * 100.0));
				}
			}
		}
		egui::CentralPanel::default().frame(egui::Frame::new().fill(p.bg)).show(ctx, |ui| {
			let full = ui.max_rect();
			let x0 = full.min.x + 22.0;
			let w = full.width() - 44.0;
			let mut y = full.min.y + self.top_bar(ui, ctx, full) + 16.0;
			self.header(ui, ctx, x0, w, y);
			y += 16.0 + 5.0 + 36.0 + 14.0;
			let mismatch = if self.game_ok { None } else { self.data.as_ref().and_then(|d| d.game_warning.clone()) };
			if let Some(msg) = mismatch {
				let r = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x0 + w, full.max.y - 22.0));
				ui.painter().rect(r, 12, Color32::TRANSPARENT, Stroke::new(1.0, p.warn), StrokeKind::Inside);
				let c = r.center();
				ui.painter().text(Pos2::new(c.x, c.y - 52.0), Align2::CENTER_CENTER, tr("Файлы из разных игр", "Files from different games"), f_semi(20.0), p.warn);
				let mut job = egui::text::LayoutJob::single_section(
					lformat!("{msg}\nСтроки не показаны: пары в них были бы неверными. Укажите файлы одной игры.", "{msg}\nLines are not shown because their pairs would be wrong. Choose files from one game."),
					egui::TextFormat { font_id: f_body(14.0), color: p.text, line_height: Some(21.0), ..Default::default() },
				);
				job.wrap.max_width = (w - 80.0).min(760.0);
				job.halign = egui::Align::Center;
				let g = ui.fonts(|f| f.layout_job(job));
				ui.painter().galley(Pos2::new(c.x, c.y - 30.0), g, p.text);
				let br = Rect::from_center_size(Pos2::new(c.x, c.y + 50.0), Vec2::new(220.0, 36.0));
				let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(br));
				if small_button(&mut bu, &p, tr("Всё равно показать", "Show anyway"), Some("arrow-right"), 36.0, true, p.text).on_hover_text(tr("Если программа ошиблась с игрой", "If the program got the game wrong")).clicked() {
					self.game_ok = true;
				}
				return;
			}
			self.page_tabs(ui, x0, y);
			y += 34.0 + 12.0;
			if self.page == 1 {
				let r = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x0 + w, full.max.y - 18.0));
				self.checks_page(ui, ctx, r);
				return;
			}
			self.search_row(ui, x0, w, y);
			y += 42.0 + 12.0;
			self.options_row(ui, ctx, x0, w, y);
			y += 30.0 + 16.0;
			self.result_row(ui, ctx, x0, w, y);
			y += 26.0 + 10.0;
			let trect = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x0 + w, full.max.y - 22.0));
			self.table(ui, ctx, trect);
		});
		self.context_menu(ctx);
		self.drag_and_drop(ctx);
		if self.btn_open {
			self.buttons_window(ctx);
		}
		show_dialog(ctx, &p, tr("Сверка перевода", "Translation compare"), &mut self.dialog);
		if self.loading.is_some() {
			ctx.request_repaint_after(std::time::Duration::from_millis(100));
		}
	}
}

fn path_and_browse(ui: &mut egui::Ui, p: &Pal, id: &str, value: &mut String, rect: Rect, enabled: bool) -> (bool, bool) {
	let bw = small_button_width(ui, tr("Обзор", "Browse"), true);
	let frect = Rect::from_min_max(rect.min, Pos2::new(rect.max.x - bw - 8.0, rect.max.y));
	let mut r = ui.new_child(egui::UiBuilder::new().max_rect(frect));
	let ch = path_input(&mut r, p, id, value, frect.size(), tr("Папка с Яп/Англ/Рус файлами", "Folder with JP/EN/RU files"), enabled);
	let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(Pos2::new(frect.max.x + 8.0, rect.min.y), rect.max)));
	let cl = small_button(&mut bu, p, tr("Обзор", "Browse"), Some("folder-open"), rect.height(), enabled, p.text).clicked();
	(ch, cl)
}

fn build_date() -> String {
	let Some(secs) = std::env::current_exe().ok().and_then(|e| std::fs::metadata(e).ok()).and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64) else { return "—".into() };
	let z = secs.div_euclid(86400) + 719468;
	let era = z.div_euclid(146097);
	let doe = z - era * 146097;
	let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
	let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
	let mp = (5 * doy + 2) / 153;
	let d = doy - (153 * mp + 2) / 5 + 1;
	let m = if mp < 10 { mp + 3 } else { mp - 9 };
	let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
	format!("{d:02}.{m:02}.{y}")
}

fn display_name(p: &compare::Pair) -> String {
	if compare::ed7().is_some() {
		if let Some(n) = p.ru.as_ref().or(p.en.as_ref()).and_then(|s| s.path.file_name()) {
			return n.to_string_lossy().to_string();
		}
	}
	p.file.rsplit('/').next().unwrap_or(&p.file).to_string()
}

pub fn app_icon() -> egui::IconData {
	icon_from(include_bytes!("icons/crossbell_compare.png"))
}

const TITLE_ED7_MAIN: &str = concat!("Translation compare (CrossbellCompare) ", env!("VER_CROSSBELLCOMPARE"), env!("VER_SUFFIX"), " — Trails from Zero / Trails to Azure");

pub fn run_ed7_main() {
	struct Wrap {
		app: App,
		cache: (PathBuf, Option<crossbell_tool::scena::Title>),
	}
	impl eframe::App for Wrap {
		fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
			let en = self.app.en_path();
			if self.cache.0 != en {
				let t = crossbell_tool::scena::detect_title(&en).or_else(|| crossbell_tool::scena::detect_title(&self.app.ru_dir()));
				self.cache = (en, t);
			}
			compare::set_ed7(Some(self.cache.1.unwrap_or(crossbell_tool::scena::Title::Zero)));
			self.app.show(ctx);
		}
		fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
			self.app.on_exit_save();
		}
	}
	compare::set_ed7(Some(crossbell_tool::scena::Title::Zero));
	let opts = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default().with_title(TITLE_ED7_MAIN).with_inner_size([1180.0, 800.0]).with_min_inner_size([960.0, 600.0]).with_drag_and_drop(true).with_icon(icon_from(include_bytes!("icons/crossbell_compare.png"))),
		centered: true,
		..Default::default()
	};
	if let Err(e) = eframe::run_native(TITLE_ED7_MAIN, opts, Box::new(|cc| Ok(Box::new(Wrap { app: App::new(cc), cache: (PathBuf::from("\0"), None) })))) {
		eprintln!("{e}");
	}
}


fn read_set(path: &std::path::Path, lower: bool) -> HashSet<String> {
	std::fs::read_to_string(path)
		.map(|s| s.lines().map(|l| l.trim_start_matches('\u{feff}').trim_end()).filter(|l| !l.trim().is_empty() && !l.starts_with('#')).map(|l| if lower { l.to_lowercase() } else { l.to_string() }).collect())
		.unwrap_or_default()
}

fn write_set(path: &std::path::Path, head: &str, set: &HashSet<String>) {
	let mut v: Vec<&String> = set.iter().collect();
	v.sort();
	let mut s = format!("\u{feff}# {head}\r\n");
	for x in v {
		s.push_str(x);
		s.push_str("\r\n");
	}
	if let Some(d) = path.parent() {
		let _ = std::fs::create_dir_all(d);
	}
	let _ = std::fs::write(path, s);
}

fn kinds() -> [(&'static str, &'static str); 3] {
	[("json", tr("диалоги .json", "dialogues .json")), ("xlsx", tr("таблицы .xlsx", "tables .xlsx")), ("yaml", tr("интерфейс .yaml", "interface .yaml"))]
}

fn kind_of(path: &str) -> usize {
	let l = path.to_lowercase();
	if l.contains(".xlsx") || l.contains(".tbl") || l.contains("._dt") {
		1
	} else if l.contains(".yaml") || l.contains(".yml") {
		2
	} else {
		0
	}
}

struct Row {
	cat: usize,
	pair: usize,
	count: usize,
	what: String,
	en: String,
	ru: String,
	file: String,
	files: String,
	line: usize,
	kind: usize,
	key: String,
}

impl App {
	fn page_tabs(&mut self, ui: &mut egui::Ui, x0: f32, y: f32) {
		let p = self.pal;
		let mut tx = x0;
		let n_err = if self.report.is_some() { Some(self.visible_rows().iter().filter(|x| checks::cats()[x.cat].2 == 0).count()) } else { None };
		for (i, t) in [tr("Сверка", "Compare"), tr("Проверки", "Checks")].iter().enumerate() {
			let label = if i == 1 { match n_err { Some(n) if n > 0 => lformat!("Проверки · ошибок {n}", "Checks · {n} errors"), _ => t.to_string() } } else { t.to_string() };
			let lw = ui.fonts(|f| f.layout_no_wrap(label.clone(), f_semi(14.0), p.text).size().x);
			let tw = lw + 28.0 + 22.0;
			let r = Rect::from_min_size(Pos2::new(tx, y), Vec2::new(tw, 34.0));
			let resp = ui.interact(r, ui.id().with(("page", i)), Sense::click());
			if self.page == i {
				ui.painter().rect_filled(r, 8, p.sel);
			} else if resp.hovered() {
				ui.painter().rect_filled(r, 8, p.div2);
			}
			let c = if self.page == i { p.text } else { p.muted };
			paint_icon(ui, if i == 0 { "columns" } else { "check" }, Rect::from_center_size(Pos2::new(r.min.x + 14.0 + 8.0, r.center().y), Vec2::splat(16.0)), c);
			ui.painter().text(Pos2::new(r.min.x + 14.0 + 22.0, r.center().y), Align2::LEFT_CENTER, &label, f_semi(14.0), c);
			if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
				self.page = i;
				self.auto_anchor = None;
			}
			tx += tw + 6.0;
		}
	}

	fn rows_all(&self, r: &checks::Report) -> Vec<Row> {
		let Some(d) = self.data.as_ref() else { return Vec::new() };
		let mut out = Vec::new();
		let mut groups: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
		for f in &r.findings {
			if f.pair == checks::NO_PAIR {
				let key = format!("Служебные коды\tфайл\t{}", f.what);
				let (path, err) = f.what.split_once(": ").unwrap_or(("", f.what.as_str()));
				let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
				let line = err.split(" at line ").nth(1).and_then(|x| x.split_whitespace().next()).and_then(|x| x.parse().ok()).unwrap_or(0);
				let what = if err.contains("escape") { tr("файл не читается: «\\» без n или другой неверный «\\»", "the file can't be read: a '\\' without n or another invalid '\\'").to_string() } else { lformat!("файл не читается: {err}", "the file can't be read: {err}") };
				let ru = if line > 0 { lformat!("Строка {line} в {name}. Пока ошибка не исправлена, файл не попадёт ни в сверку, ни в игру.", "Line {line} in {name}. Until the error is fixed, the file reaches neither Compare nor the game.") } else { path.to_string() };
				out.push(Row { cat: f.cat, pair: f.pair, count: 1, what, en: String::new(), ru, file: name, files: String::new(), line, kind: kind_of(&f.what), key });
				continue;
			}
			let pr = &d.pairs[f.pair];
			let en = pr.en.as_ref().map(|s| s.text.clone()).unwrap_or_default();
			let ru = pr.ru.as_ref().map(|s| s.text.clone()).unwrap_or_default();
			let line = pr.ru.as_ref().map(|s| s.line).unwrap_or(0);
			let kind = kind_of(&pr.file);
			if f.cat == checks::CAT_UNTR {
				let shown = if f.what.is_empty() { &en } else { &ru };
				let t = shown.trim().to_string();
				if let Some(&gi) = groups.get(&format!("{kind}\t{t}")) {
					let g: &mut Row = &mut out[gi];
					g.count += 1;
					let name = display_name(pr);
					if !g.files.split(", ").any(|x| x == name) && g.files.matches(", ").count() < 3 {
						g.files.push_str(", ");
						g.files.push_str(&name);
					}
					continue;
				}
				groups.insert(format!("{kind}\t{t}"), out.len());
				out.push(Row { cat: f.cat, pair: f.pair, count: 1, what: String::new(), en: compare::clean(shown, false), ru: compare::clean(&ru, false), file: display_name(pr), files: display_name(pr), line, kind, key: format!("Не переведено\t\t{t}") });
			} else {
				out.push(Row { cat: f.cat, pair: f.pair, count: 1, what: f.what.clone(), en: compare::clean(&en, false), ru: compare::clean(&ru, false), file: display_name(pr), files: String::new(), line, kind, key: format!("{}\t{}\t{}", checks::CAT_KEYS[f.cat], pr.file, pr.id) });
			}
		}
		out
	}

	fn rows_for(&self, r: &checks::Report) -> Vec<Row> {
		self.rows_all(r).into_iter().filter(|x| !self.ignored.contains(&x.key)).collect()
	}

	fn visible_rows(&mut self) -> std::rc::Rc<Vec<Row>> {
		if self.rows_cache.is_none() {
			let v = match &self.report {
				Some(r) => self.rows_for(r),
				None => Vec::new(),
			};
			self.rows_cache = Some(std::rc::Rc::new(v));
		}
		self.rows_cache.clone().unwrap_or_default()
	}

	fn run_checks(&mut self) {
		if let Some(d) = self.data.as_ref() {
			self.report = Some(checks::run(d));
		}
		self.rows_cache = None;
		self.rows_all_cache = None;
	}

	fn export_checks(&mut self, rows: &[Row]) {
		let start = self.en_dir().display().to_string();
		let Some(path) = save_file(&start, tr("Выгрузить проверки в Excel", "Export checks to Excel"), tr("Проверки.xlsx", "Checks.xlsx"), tr("Книга Excel", "Excel workbook"), &["xlsx"]) else { return };
		let kinds = self.kinds;
		let data = self.data.as_ref();
		let res = (|| -> Result<usize, rust_xlsxwriter::XlsxError> {
			use rust_xlsxwriter::{Color, Format, FormatAlign, Workbook};
			let cut = |s: &str| -> String { s.chars().take(32000).collect() };
			let head = Format::new().set_bold().set_background_color(Color::RGB(0xDDEBF7));
			let hit = Format::new().set_text_wrap().set_align(FormatAlign::Top).set_background_color(Color::RGB(0xFFF2CC));
			let ctx_f = Format::new().set_text_wrap().set_align(FormatAlign::Top).set_font_color(Color::RGB(0x808080));
			const AROUND: usize = 5;
			let raw = |s: &Option<compare::Side>| s.as_ref().map(|s| s.text.clone()).unwrap_or_default();
			let line_of = |p: &compare::Pair| p.ru.as_ref().or(p.en.as_ref()).map(|s| s.line).unwrap_or(0);
			let mut wb = Workbook::new();
			let mut n = 0;
			for (c, (title, _, _)) in checks::cats().iter().enumerate() {
				let items: Vec<&Row> = rows.iter().filter(|x| x.cat == c && kinds[x.kind]).collect();
				if items.is_empty() {
					continue;
				}
				let ws = wb.add_worksheet();
				ws.set_name(*title)?;
				for (k, h) in [tr("Файл", "File"), tr("Строка", "Line"), tr("Что не так", "What's wrong"), tr("Яп, Англ, Рус", "JP, EN, RU"), tr("Яп, Англ, Рус", "JP, EN, RU"), tr("Повторов", "Repeats"), tr("Где ещё", "Also in")].iter().enumerate() {
					ws.write_string_with_format(0, k as u16, *h, &head)?;
				}
				for (k, w) in [22.0, 8.0, 40.0, 60.0, 60.0, 10.0, 40.0].iter().enumerate() {
					ws.set_column_width(k as u16, *w)?;
				}
				ws.set_freeze_panes(1, 0)?;
				let mut r = 1u32;
				for it in items.iter() {
					let range = data.filter(|_| it.pair != checks::NO_PAIR).and_then(|d| {
						let file = &d.pairs.get(it.pair)?.file;
						let &(a, b) = d.files.get(file)?;
						Some((d, it.pair.saturating_sub(AROUND).max(a), (it.pair + AROUND + 1).min(b)))
					});
					match range {
						Some((d, a, b)) => {
							for k in a..b {
								let p = &d.pairs[k];
								let me = k == it.pair;
								let f = if me { &hit } else { &ctx_f };
								ws.write_string_with_format(r, 0, &cut(&it.file), f)?;
								ws.write_number_with_format(r, 1, line_of(p) as f64, f)?;
								ws.write_string_with_format(r, 2, if me { cut(&it.what) } else { String::new() }, f)?;
								ws.write_string_with_format(r, 3, &cut(&raw(&p.en)), f)?;
								ws.write_string_with_format(r, 4, &cut(&raw(&p.ru)), f)?;
								if me {
									ws.write_number_with_format(r, 5, it.count as f64, f)?;
									ws.write_string_with_format(r, 6, &cut(&it.files), f)?;
								}
								r += 1;
							}
						}
						None => {
							ws.write_string_with_format(r, 0, &cut(&it.file), &hit)?;
							ws.write_number_with_format(r, 1, it.line as f64, &hit)?;
							ws.write_string_with_format(r, 2, &cut(&it.what), &hit)?;
							ws.write_string_with_format(r, 3, &cut(&it.en), &hit)?;
							ws.write_string_with_format(r, 4, &cut(&it.ru), &hit)?;
							ws.write_number_with_format(r, 5, it.count as f64, &hit)?;
							ws.write_string_with_format(r, 6, &cut(&it.files), &hit)?;
							r += 1;
						}
					}
					r += 1;
					n += 1;
				}
			}
			wb.save(&path)?;
			Ok(n)
		})();
		match res {
			Ok(n) => self.flash(&lformat!("Выгружено находок: {n}", "Findings exported: {n}")),
			Err(e) => self.dialog = Dialog::Info(lformat!("Не удалось записать {}:\n{e}\n\nФайл открыт в Excel? Закройте его и повторите.", "Could not write {}:\n{e}\n\nIs the file open in Excel? Close it and try again.", path.display())),
		}
	}

	fn all_rows(&mut self) -> std::rc::Rc<Vec<Row>> {
		if self.rows_all_cache.is_none() {
			let v = match &self.report {
				Some(r) => self.rows_all(r),
				None => Vec::new(),
			};
			self.rows_all_cache = Some(std::rc::Rc::new(v));
		}
		self.rows_all_cache.clone().unwrap_or_default()
	}

	fn open_pair(&mut self, idx: usize) {
		let Some(d) = self.data.as_ref() else { return };
		let p = &d.pairs[idx];
		let Some(&(a, b)) = d.files.get(&p.file) else { return };
		let name = display_name(p);
		let file_sel = d.files.keys().position(|k| *k == p.file).map(|n| n + 1).unwrap_or(0);
		self.rows = (a..b).collect();
		self.total = b - a;
		self.found = lformat!("Файл {name}: {} — выделена строка из «Проверок»", "File {name}: {}, the line from Checks is selected", plural(b - a, tr("строка", "line"), tr("строки", "lines"), tr("строк", "lines")));
		self.chip = None;
		self.searched = true;
		self.inline = None;
		self.query = String::new();
		self.file_sel = file_sel;
		self.reset_layouts();
		self.selected = Some(idx - a);
		self.scroll_to = Some(idx - a);
		self.page = 0;
		self.auto_anchor = None;
	}

	fn checks_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: Rect) {
		let p = self.pal;
		if self.data.is_none() {
			let msg = if self.loading.is_some() { tr("Загрузка…", "Loading…") } else { tr("Укажите папки и нажмите «Загрузить» — проверки запустятся сами", "Choose the folders and click 'Load': the checks start on their own") };
			ui.painter().rect(rect, 12, Color32::TRANSPARENT, Stroke::new(1.0, p.div), StrokeKind::Inside);
			ui.painter().text(rect.center(), Align2::CENTER_CENTER, msg, f_body(14.0), p.muted);
			return;
		}
		if self.report.is_none() {
			self.run_checks();
		}
		let rows = self.visible_rows();
		let checked = self.report.as_ref().map(|r| r.checked).unwrap_or(0);
		let counts: Vec<usize> = (0..checks::cats().len()).map(|c| rows.iter().filter(|x| x.cat == c).count()).collect();
		let sev_col = |s: u8| match s { 0 => p.warn, 1 => Color32::from_rgb(0xd0, 0x8a, 0x2e), _ => p.accent };
		let sev_txt = |c: usize| match checks::cats()[c].2 {
			0 => tr("ошибка — может сломать текст или игру", "error: may break the text or the game"),
			1 => tr("пропуск — одинаковые тексты одной строкой", "gap: identical texts on one line"),
			_ => tr("проверить — бывает и верно", "check: may also be correct"),
		};

		let lw = 340.0;
		let left = Rect::from_min_max(rect.min, Pos2::new(rect.min.x + lw, rect.max.y));
		let mut y = left.min.y;
		let flash = self.flash.as_ref().filter(|(_, t)| t.elapsed().as_secs_f32() < 1.5).map(|(m, _)| m.clone());
		if let Some(m) = flash {
			ui.painter().text(Pos2::new(left.min.x, y + 14.0), Align2::LEFT_CENTER, m, f_body(12.0), p.ok_strong);
			ctx.request_repaint_after(std::time::Duration::from_millis(150));
		} else {
			ui.painter().galley(Pos2::new(left.min.x, y + 8.0), ui.fonts(|f| f.layout_job(caps_job(&lformat!("Проверено: {}", "Checked: {}", plural(checked, tr("реплика", "line"), tr("реплики", "lines"), tr("реплик", "lines"))), p.muted))), p.muted);
		}
		{
			let br = Rect::from_min_size(Pos2::new(left.max.x - 150.0, y), Vec2::new(150.0, 30.0));
			let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(br));
			if primary_button_icon(&mut bu, &p, tr("Проверить заново", "Check again"), Some("refresh"), br.size(), 8, f_semi(13.0), true).clicked() {
				self.ignored = read_set(&ign_file(), false);
				self.run_checks();
				self.flash(tr("Проверено заново", "Checked again"));
				return;
			}
		}
		y += 40.0;
		for (c, (title, _, sev)) in checks::cats().iter().enumerate() {
			let r =Rect::from_min_size(Pos2::new(left.min.x, y), Vec2::new(lw, 50.0));
			let resp = ui.interact(r, ui.id().with(("cat", c)), Sense::click());
			let on = self.cat == c;
			ui.painter().rect(r, 10, if on { p.sel } else if resp.hovered() { p.div2 } else { Color32::TRANSPARENT }, Stroke::new(1.0, if on { p.accent } else { p.div }), StrokeKind::Inside);
			let col = sev_col(*sev);
			ui.painter().circle_filled(Pos2::new(r.min.x + 17.0, r.center().y), 5.0, col);
			ui.painter().text(Pos2::new(r.min.x + 32.0, r.min.y + 16.0), Align2::LEFT_CENTER, *title, f_semi(14.0), p.text);
			ui.painter().text(Pos2::new(r.min.x + 32.0, r.min.y + 35.0), Align2::LEFT_CENTER, sev_txt(c), f_body(12.0), p.muted);
			ui.painter().text(Pos2::new(r.max.x - 14.0, r.center().y), Align2::RIGHT_CENTER, group(counts[c]), f_semi(18.0), if counts[c] == 0 { p.ok_strong } else { col });
			if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
				self.cat = c;
				self.auto_anchor = None;
			}
			y += 50.0 + 6.0;
		}
		{
			let r = Rect::from_min_size(Pos2::new(left.min.x, y + 4.0), Vec2::new(lw, 40.0));
			let resp = ui.interact(r, ui.id().with("hidden_btn"), Sense::click());
			let st = Stroke::new(1.0, if resp.hovered() { p.accent } else { p.div });
			ui.painter().extend(egui::Shape::dashed_line(&[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()], st, 6.0, 4.0));
			ui.painter().text(Pos2::new(r.min.x + 14.0, r.center().y), Align2::LEFT_CENTER, tr("Скрытые — «Не ошибка»", "Hidden: 'Not an error'"), f_body(13.5), p.text);
			ui.painter().text(Pos2::new(r.max.x - 14.0, r.center().y), Align2::RIGHT_CENTER, group(self.ignored.len()), f_body(13.5), p.muted);
			if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
				self.hidden_open = true;
			}
			y += 56.0;
		}
		{
			let br = Rect::from_min_size(Pos2::new(left.min.x, y), Vec2::new(lw, 30.0));
			let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(br));
			if small_button(&mut bu, &p, tr("Выгрузить в Excel", "Export to Excel"), Some("sheet"), 30.0, !rows.is_empty(), p.text).on_hover_text(tr("Находки всех разделов (без скрытых) — в файл .xlsx, по листу на раздел", "Findings of all sections (except hidden) into an .xlsx file, one sheet per section")).clicked() {
				self.export_checks(&rows);
			}
			y += 40.0;
		}
		{
			let mut job = egui::text::LayoutJob::single_section(
				tr("Нажмите колёсико мыши в списке и ведите мышь вверх или вниз — список поедет сам. Ещё нажатие — стоп.", "Press the mouse wheel in the list and move the mouse up or down: the list scrolls by itself. Press again to stop.").to_string(),
				egui::TextFormat { font_id: f_body(12.0), color: p.muted, line_height: Some(18.0), ..Default::default() },
			);
			job.wrap.max_width = lw;
			let g = ui.fonts(|f| f.layout_job(job));
			ui.painter().galley(Pos2::new(left.min.x, y), g, p.muted);
		}

		let right = Rect::from_min_max(Pos2::new(rect.min.x + lw + 18.0, rect.min.y), rect.max);
		ui.painter().rect(right, 12, Color32::TRANSPARENT, Stroke::new(1.0, p.div), StrokeKind::Inside);
		let (title, _, _) = checks::cats()[self.cat];
		let desc = checks::cat_desc(self.cat);
		let hx = right.min.x + 18.0;
		ui.painter().text(Pos2::new(hx, right.min.y + 22.0), Align2::LEFT_CENTER, format!("{title} · {}", group(counts[self.cat])), f_semi(16.0), p.text);
		paint_text_clipped(ui, Rect::from_min_size(Pos2::new(hx, right.min.y + 36.0), Vec2::new(right.width() - 36.0, 18.0)), desc, f_body(12.5), p.muted, Align2::LEFT_CENTER);
		{
			let cy = right.min.y + 62.0;
			ui.painter().text(Pos2::new(hx, cy + 15.0), Align2::LEFT_CENTER, tr("Показывать:", "Show:"), f_body(13.0), p.muted);
			let mut cx = hx + ui.fonts(|f| f.layout_no_wrap(tr("Показывать:", "Show:").into(), f_body(13.0), p.muted).size().x) + 10.0;
			for (k, (_, label)) in kinds().iter().enumerate() {
				let n = rows.iter().filter(|x| x.cat == self.cat && x.kind == k).count();
				let text = format!("{label}  {}", group(n));
				let tw = ui.fonts(|f| f.layout_no_wrap(text.clone(), f_body(13.0), p.text).size().x) + 46.0;
				let r = Rect::from_min_size(Pos2::new(cx, cy), Vec2::new(tw, 30.0));
				let resp = ui.interact(r, ui.id().with(("kind", k)), Sense::click());
				let on = self.kinds[k];
				ui.painter().rect(r, 15, if on { p.sel } else if resp.hovered() { p.div2 } else { Color32::TRANSPARENT }, Stroke::new(1.0, if on { p.accent } else { p.div }), StrokeKind::Inside);
				let b = Rect::from_center_size(Pos2::new(r.min.x + 19.0, r.center().y), Vec2::splat(14.0));
				if on {
					ui.painter().rect_filled(b, 4, p.accent);
					paint_icon(ui, "check", b.shrink(1.5), p.on_accent);
				} else {
					ui.painter().rect(b, 4, Color32::TRANSPARENT, Stroke::new(1.5, p.muted), StrokeKind::Inside);
				}
				ui.painter().text(Pos2::new(r.min.x + 34.0, r.center().y), Align2::LEFT_CENTER, text, f_body(13.0), if on { p.text } else { p.muted });
				if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
					self.kinds[k] = !self.kinds[k];
				}
				cx += tw + 8.0;
			}
		}
		let body = Rect::from_min_max(Pos2::new(right.min.x + 1.0, right.min.y + 104.0), Pos2::new(right.max.x - 1.0, right.max.y - 1.0));
		ui.painter().line_segment([Pos2::new(right.min.x + 1.0, body.min.y - 1.0), Pos2::new(right.max.x - 1.0, body.min.y - 1.0)], Stroke::new(1.0, p.div));
		let list_rect = Rect::from_min_max(Pos2::new(body.min.x + 10.0, body.min.y + 4.0), Pos2::new(body.max.x - BAR_W - 2.0, body.max.y - 4.0));
		let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list_rect));
		child.set_clip_rect(list_rect.expand2(Vec2::new(0.0, 4.0)));
		let mut act_open: Option<usize> = None;
		let mut act_menu: Option<(usize, Pos2)> = None;
		let mut act_hide: Vec<String> = Vec::new();

		let auto_v = self.auto_scroll(ctx, body);


		let items: Vec<&Row> = rows.iter().filter(|x| x.cat == self.cat && self.kinds[x.kind]).collect();
		enum Ent<'a> {
			Group(&'a str, Vec<&'a Row>),
			One(&'a Row, bool),
		}
		let mut ents: Vec<Ent> = Vec::new();
		if self.cat == checks::CAT_UNTR {
			ents.extend(items.iter().map(|r| Ent::One(r, false)));
		} else {
			let mut order: Vec<&str> = Vec::new();
			let mut by: HashMap<&str, Vec<&Row>> = HashMap::new();
			for r in &items {
				if !by.contains_key(r.what.as_str()) {
					order.push(r.what.as_str());
				}
				by.entry(r.what.as_str()).or_default().push(r);
			}
			order.sort_by_key(|w| std::cmp::Reverse(by[w].len()));
			for w in order {
				let v = by.remove(w).unwrap_or_default();
				if v.len() == 1 {
					ents.push(Ent::One(v[0], false));
					continue;
				}
				let open = self.open_group.as_ref().is_some_and(|(c, g)| *c == self.cat && g == w);
				ents.push(Ent::Group(w, v.clone()));
				if open {
					ents.extend(v.into_iter().map(|r| Ent::One(r, true)));
				}
			}
		}
		let mut act_group: Option<String> = None;
		let row_h = if self.cat == checks::CAT_UNTR { 64.0 } else { 86.0 };
		let mut sa = egui::ScrollArea::vertical().id_salt(("items", self.cat)).auto_shrink([false, false]).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
		if let Some(o) = self.bar_set.remove("items") {
			sa = sa.vertical_scroll_offset(o);
		}
		let out = sa.show_rows(&mut child, row_h, ents.len(), |ui, range| {
			for i in range {
				let (rr, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
				let (it, rr) = match &ents[i] {
					Ent::Group(w, v) => {
						let open = self.open_group.as_ref().is_some_and(|(c, g)| *c == self.cat && g == w);
						if resp.hovered() || open {
							ui.painter().rect_filled(rr, 6, p.div2.gamma_multiply(if open { 0.9 } else { 0.5 }));
						}
						let tw = rr.width() - 150.0;
						let tx = rr.min.x + 8.0;
						let g = ui.painter().layout_no_wrap(w.to_string(), f_mono(12.0), p.warn);
						let wr = Rect::from_min_size(Pos2::new(tx, rr.min.y + 8.0), Vec2::new((g.size().x + 14.0).min(tw - 140.0), 20.0));
						ui.painter().rect_filled(wr, 4, p.warn.gamma_multiply(0.16));
						paint_text_clipped(ui, wr.shrink2(Vec2::new(7.0, 0.0)), w, f_mono(12.0), p.warn, Align2::LEFT_CENTER);
						ui.painter().text(Pos2::new(wr.max.x + 10.0, wr.center().y), Align2::LEFT_CENTER, lformat!("{} реплик", "{} lines", group(v.len())), f_semi(14.0), p.warn);
						let mut files: Vec<&str> = Vec::new();
						for r in v {
							if !files.contains(&r.file.as_str()) {
								files.push(r.file.as_str());
							}
						}
						let more = if files.len() > 4 { lformat!(" и ещё {} файлов", " and {} more files", files.len() - 4) } else { String::new() };
						let wh = format!("{}{more}", files.iter().take(4).copied().collect::<Vec<_>>().join(", "));
						paint_text_clipped(ui, Rect::from_min_size(Pos2::new(tx, rr.min.y + 32.0), Vec2::new(tw, 18.0)), &wh, f_body(12.0), p.muted, Align2::LEFT_CENTER);
						paint_text_clipped(ui, Rect::from_min_size(Pos2::new(tx, rr.min.y + 54.0), Vec2::new(tw, 18.0)), &lformat!("например: {}", "for example: {}", v[0].ru), f_body(12.5), p.text, Align2::LEFT_CENTER);
						let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(Pos2::new(rr.max.x - 140.0, rr.min.y + 6.0), Pos2::new(rr.max.x - 4.0, rr.max.y - 6.0))).layout(egui::Layout::top_down(egui::Align::Max)));
						bu.spacing_mut().item_spacing.y = 6.0;
						if small_button(&mut bu, &p, if open { tr("Свернуть", "Collapse") } else { tr("Показать все", "Show all") }, Some(if open { "chevron-up" } else { "chevron-down" }), 28.0, true, p.accent).clicked() || resp.clicked() {
							act_group = Some(w.to_string());
						}
						if small_button(&mut bu, &p, tr("Не ошибка · все", "Not an error · all"), Some("eye-off"), 26.0, true, p.muted).on_hover_text(tr("Убрать из списка все реплики группы (вернуть — «Скрытые»)", "Remove all the group's lines from the list (restore them in 'Hidden')")).clicked() {
							act_hide.extend(v.iter().map(|r| r.key.clone()));
						}
						ui.painter().line_segment([Pos2::new(rr.min.x, rr.max.y), Pos2::new(rr.max.x, rr.max.y)], Stroke::new(1.0, p.div2));
						continue;
					}
					Ent::One(r, true) => {
						ui.painter().rect_filled(Rect::from_min_size(Pos2::new(rr.min.x + 8.0, rr.min.y + 4.0), Vec2::new(3.0, rr.height() - 8.0)), 1, p.warn.gamma_multiply(0.6));
						(*r, rr.with_min_x(rr.min.x + 24.0))
					}
					Ent::One(r, false) => (*r, rr),
				};
				if resp.hovered() {
					ui.painter().rect_filled(rr, 6, p.div2.gamma_multiply(0.5));
				}
				let tw = rr.width() - 150.0;
				if it.cat == checks::CAT_UNTR {
					let tx = rr.min.x + 8.0;
					let cnt = format!(" ({})", group(it.count));
					let cw = ui.fonts(|f| f.layout_no_wrap(cnt.clone(), f_semi(14.0), p.text).size().x);
					let t = if it.en.is_empty() { it.ru.clone() } else { it.en.clone() };
					let tg = ui.fonts(|f| f.layout_no_wrap(t.clone(), f_mono(13.5), p.text).size().x).min(tw - cw - 20.0);
					paint_text_clipped(ui, Rect::from_min_size(Pos2::new(tx, rr.min.y + 10.0), Vec2::new(tg, 20.0)), &t, f_mono(13.5), p.text, Align2::LEFT_CENTER);
					ui.painter().text(Pos2::new(tx + tg, rr.min.y + 20.0), Align2::LEFT_CENTER, cnt, f_semi(14.0), Color32::from_rgb(0xd0, 0x8a, 0x2e));
					let wh = if it.count == 1 { lformat!("{} · строка {}", "{} · line {}", it.file, it.line) } else { it.files.clone() };
					paint_text_clipped(ui, Rect::from_min_size(Pos2::new(tx, rr.min.y + 34.0), Vec2::new(tw - 8.0, 18.0)), &wh, f_body(12.0), p.muted, Align2::LEFT_CENTER);
				} else {
					let fx = rr.min.x + 8.0;
					if it.pair == checks::NO_PAIR && it.line == 0 {
						ui.painter().text(Pos2::new(fx, rr.min.y + 16.0), Align2::LEFT_CENTER, tr("файл", "file"), f_mono(12.5), p.text);
					} else {
						paint_text_clipped(ui, Rect::from_min_size(Pos2::new(fx, rr.min.y + 7.0), Vec2::new(116.0, 18.0)), &it.file, f_mono(12.5), p.text, Align2::LEFT_CENTER);
						ui.painter().text(Pos2::new(fx, rr.min.y + 34.0), Align2::LEFT_CENTER, lformat!("строка {}", "line {}", it.line), f_body(11.5), p.muted);
					}
					let tx = rr.min.x + 130.0;
					let tw2 = tw - 130.0;
					let mut ty = rr.min.y + 8.0;
					let nl = it.what.starts_with('\\') || it.pair == checks::NO_PAIR;
					let chip_col = if nl { Color32::from_rgb(0xd0, 0x8a, 0x2e) } else { p.warn };
					let g = ui.painter().layout_no_wrap(it.what.clone(), f_mono(12.0), chip_col);
					let wr = Rect::from_min_size(Pos2::new(tx, ty), Vec2::new((g.size().x + 14.0).min(tw2), 20.0));
					ui.painter().rect_filled(wr, 4, chip_col.gamma_multiply(0.16));
					paint_text_clipped(ui, wr.shrink2(Vec2::new(7.0, 0.0)), &it.what, f_mono(12.0), chip_col, Align2::LEFT_CENTER);
					ty += 24.0;
					if !it.en.is_empty() {
						ui.painter().text(Pos2::new(tx, ty + 9.0), Align2::LEFT_CENTER, "EN", f_semi(11.5), p.muted);
						paint_text_clipped(ui, Rect::from_min_size(Pos2::new(tx + 28.0, ty), Vec2::new(tw2 - 28.0, 18.0)), &it.en, f_body(12.5), p.muted, Align2::LEFT_CENTER);
						ty += 22.0;
						ui.painter().text(Pos2::new(tx, ty + 9.0), Align2::LEFT_CENTER, "RU", f_semi(11.5), p.accent);
						paint_text_clipped(ui, Rect::from_min_size(Pos2::new(tx + 28.0, ty), Vec2::new(tw2 - 28.0, 18.0)), &it.ru, f_body(12.5), p.text, Align2::LEFT_CENTER);
					} else {
						paint_text_clipped(ui, Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(tw2, 36.0)), &it.ru, f_body(12.5), p.text, Align2::LEFT_CENTER);
					}
				}
				let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(Pos2::new(rr.max.x - 140.0, rr.min.y + 6.0), Pos2::new(rr.max.x - 4.0, rr.max.y - 6.0))).layout(egui::Layout::top_down(egui::Align::Max)));
				bu.spacing_mut().item_spacing.y = 6.0;
				if it.pair != checks::NO_PAIR && (small_button(&mut bu, &p, tr("Открыть в сверке", "Open in Compare"), Some("external"), 28.0, true, p.accent).clicked() || resp.double_clicked()) {
					act_open = Some(it.pair);
				}
				if it.pair != checks::NO_PAIR && resp.secondary_clicked() {
					act_menu = resp.interact_pointer_pos().map(|pp| (it.pair, pp));
				}
				if small_button(&mut bu, &p, tr("Не ошибка", "Not an error"), Some("eye-off"), 26.0, true, p.muted).on_hover_text(tr("Убрать из списка (вернуть — «Скрытые»)", "Remove from the list (restore it in 'Hidden')")).clicked() {
					act_hide.push(it.key.clone());
				}
				ui.painter().line_segment([Pos2::new(rr.min.x, rr.max.y), Pos2::new(rr.max.x, rr.max.y)], Stroke::new(1.0, p.div2));
			}
		});
		if auto_v != 0.0 {
			let max_off = (out.content_size.y - out.inner_rect.height()).max(0.0);
			self.bar_set.insert("items", (out.state.offset.y + auto_v).clamp(0.0, max_off));
		}
		self.checks_off = out.state.offset.y;
		if let Some(o) = vbar(ui, &p, "items", Rect::from_min_max(Pos2::new(body.max.x - BAR_W, body.min.y + 6.0), Pos2::new(body.max.x - 5.0, body.max.y - 6.0)), out.state.offset.y, out.content_size.y, out.inner_rect.height()) {
			self.bar_set.insert("items", o);
			self.auto_anchor = None;
			ctx.request_repaint();
		}
		if items.is_empty() {
			ui.painter().text(body.center(), Align2::CENTER_CENTER, tr("Ничего не найдено", "Nothing found"), f_body(14.0), p.ok_strong);
		}
		self.auto_cursor(ctx, auto_v);
		let all_rows = if self.hidden_open { self.all_rows() } else { std::rc::Rc::new(Vec::new()) };
		if act_menu.is_some() {
			self.check_menu = act_menu;
		}
		if let Some(i) = act_open {
			self.open_pair(i);
			ctx.request_repaint();
		}
		if let Some(w) = act_group {
			let same = self.open_group.as_ref().is_some_and(|(c, g)| *c == self.cat && *g == w);
			self.open_group = if same { None } else { Some((self.cat, w)) };
		}
		if !act_hide.is_empty() {
			self.ignored.extend(act_hide);
			self.rows_cache = None;
			write_set(&ign_file(), tr("скрытые кнопкой «Не ошибка» (раздел, файл, строка)", "hidden with the 'Not an error' button (section, file, line)"), &self.ignored);
		}
		if self.hidden_open {
			self.hidden_window(ctx, &all_rows);
		}
	}

	fn buttons_window(&mut self, ctx: &egui::Context) {
		use crate::kai_buttons::{self as kb, pad_table};
		let devs = kb::devices();
		#[allow(non_snake_case)]
		let DEVICES = devs;
		let p = self.pal;
		self.btn.ensure_loaded();
		if self.btn_codes.is_none() {
			if let Some(d) = self.data.as_ref() {
				self.btn_codes = Some(kb::scan(d.pairs.iter().filter_map(|x| x.ru.as_ref().map(|s| s.text.clone()))));
			}
		}
		let codes: Vec<(u32, usize, String)> = self.btn_codes.as_ref().map(|m| m.iter().map(|(k, v)| (*k, v.0, v.1.clone())).collect()).unwrap_or_default();
		let actions: Vec<&(u32, usize, String)> = codes.iter().filter(|x| !kb::is_atlas(x.0)).collect();
		let n_atlas = codes.iter().filter(|x| kb::is_atlas(x.0)).count();
		let ed7 = compare::ed7().is_some();
		if self.btn_sel.is_none() {
			self.btn_sel = actions.first().map(|x| x.0);
		}
		let mut close = false;
		let mut changed = false;
		let screen = ctx.screen_rect();
		egui::Area::new(egui::Id::new("btn_dim")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
			let (r, _) = ui.allocate_exact_size(screen.size(), Sense::click());
			ui.painter().rect_filled(r, 0, Color32::from_black_alpha(if p.dark { 120 } else { 70 }));
		});
		let w = 1000f32.min(screen.width() - 40.0);
		let h = 760f32.min(screen.height() - 30.0);
		let rect = Rect::from_center_size(screen.center(), Vec2::new(w, h));
		egui::Area::new(egui::Id::new("btn_box")).order(egui::Order::Foreground).fixed_pos(rect.min).show(ctx, |ui| {
			let _ = ui.allocate_exact_size(rect.size(), Sense::hover());
			let painter = ui.painter().clone();
			painter.add(egui::epaint::Shadow { offset: [0, 14], blur: 40, spread: 0, color: Color32::from_black_alpha(70) }.as_shape(rect, 14));
			painter.rect(rect, 14, p.bg, Stroke::new(1.0, p.div), StrokeKind::Inside);
			let x0 = rect.min.x + 22.0;
			let iw = rect.width() - 44.0;
			painter.text(Pos2::new(x0, rect.min.y + 26.0), Align2::LEFT_CENTER, tr("Значки в русском тексте", "Icons in the Russian text"), f_semi(18.0), p.text);
			painter.text(Pos2::new(x0, rect.min.y + 50.0), Align2::LEFT_CENTER, if ed7 { tr("Коды #…I и #…M показываются картинками из текстур игры — только здесь, в Сверке (английский и русский столбцы). В файлы перевода ничего не пишется.", "#…I and #…M codes are shown as images from the game's textures, only here in Compare (English and Russian columns). Nothing is written to the translation files.") } else { tr("Коды <I…> показываются картинками из текстур игры — только здесь, в Сверке (английский и русский столбцы). В файлы перевода ничего не пишется.", "<I…> codes are shown as images from the game's textures, only here in Compare (English and Russian columns). Nothing is written to the translation files.") }, f_body(12.5), p.muted);
			let mut y = rect.min.y + 70.0;
			{
				let mut r = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::new(x0, y), Vec2::new(iw, 30.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
				let mut on = self.btn.on;
				if checkbox(&mut r, &p, &mut on, tr("Показывать значки в тексте", "Show icons in the text"), 16.0, f_body(13.5)).changed() {
					self.btn.on = on;
					changed = true;
				}
			}
			y += 38.0;
			{
				painter.galley(Pos2::new(x0, y), ui.fonts(|f| f.layout_job(caps_job(tr("Значки и кнопки какой игры показывать", "Which game's icons and buttons to show"), p.muted))), p.muted);
				y += 20.0;
				let gap = 12.0;
				let games: &[(&str, &str)] = if ed7 { &crate::kai_buttons::ED7_GAMES } else { &crate::kai_buttons::KAI_GAMES };
				let cw = (iw - gap * (games.len() as f32 - 1.0)) / games.len() as f32;
				let ch = 54.0;
				for (gi, (name, key)) in games.iter().enumerate() {
					let r = Rect::from_min_size(Pos2::new(x0 + gi as f32 * (cw + gap), y), Vec2::new(cw, ch));
					let resp = ui.interact(r, egui::Id::new(("btn_game", gi)), Sense::click());
					let sel = self.btn.game_dir.trim() == *key;
					let fill = if sel { p.sel } else if resp.hovered() { p.div2 } else { p.bg };
					painter.rect(r, 10, fill, Stroke::new(if sel { 2.0 } else { 1.0 }, if sel { p.accent_strong } else { p.div }), StrokeKind::Inside);
					let c = Pos2::new(r.min.x + 22.0, r.min.y + 19.0);
					if sel {
						painter.circle_filled(c, 7.0, p.accent_strong);
						painter.circle_filled(c, 3.0, p.bg);
					} else {
						painter.circle_stroke(c, 6.5, Stroke::new(1.5, p.muted));
					}
					painter.text(Pos2::new(r.min.x + 38.0, r.min.y + 19.0), Align2::LEFT_CENTER, *name, f_semi(15.0), p.text);
					painter.text(Pos2::new(r.min.x + 38.0, r.min.y + 39.0), Align2::LEFT_CENTER, lformat!("значки и кнопки из {name}", "icons and buttons from {name}"), f_body(12.0), p.muted);
					if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !sel {
						self.btn.game_dir = key.to_string();
						changed = true;
					}
				}
				y += ch + 8.0;
			}
			let st_col = if self.btn.ready() { p.muted } else { p.warn };
			paint_text_clipped(ui, Rect::from_min_size(Pos2::new(x0, y), Vec2::new(iw, 18.0)), &self.btn.status, f_body(12.0), st_col, Align2::LEFT_CENTER);
			y += 26.0;
			{
				let mut r = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::new(x0, y), Vec2::new(iw, 32.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
				r.label(egui::RichText::new(tr("Устройство", "Device")).font(f_body(13.0)).color(p.muted));
				r.add_space(10.0);
				let mut dev = self.btn.device;
				let tips = if compare::ed7().is_some() { ["", "", "", tr("набор с кнопкой «•••»", "the set with the '•••' button"), tr("кнопки DualSense (buttons_ps5)", "DualSense buttons (buttons_ps5)"), tr("клавиши и мышь назначаются отдельно от геймпада", "keys and mouse are assigned separately from the gamepad")] } else { ["", "", "", tr("набор с кнопкой «•••»", "the set with the '•••' button"), tr("набор Stadia: кнопки L1/R1 прямоугольной формы", "Stadia set: rectangular L1/R1 buttons"), tr("клавиши назначаются отдельно от геймпада", "keys are assigned separately from the gamepad")] };
				if segmented(&mut r, &p, "btn_dev", &DEVICES, &mut dev, None, 30.0, 7, f_body(13.0), &tips) {
					self.btn.device = dev;
					self.btn.changed();
					changed = true;
				}
			}
			y += 42.0;
			painter.text(Pos2::new(x0, y + 8.0), Align2::LEFT_CENTER, if ed7 { lformat!("Значков c_icon1 и предметов в тексте: {} — показываются сами. Ниже кнопки (#160I и выше): взяты с экрана настроек игры, остальные назначьте.", "c_icon1 and item icons in the text: {}, shown on their own. Buttons below (#160I and up): taken from the game's settings screen, assign the rest.", n_atlas) } else { lformat!("Значков icons.png в тексте: {} — показываются сами. Ниже коды действий 900+: часть кнопок взята из файлов игры, остальные назначьте.", "icons.png icons in the text: {}, shown on their own. Action codes 900+ below: some buttons come from the game's files, assign the rest.", n_atlas) }, f_body(12.5), p.text);
			y += 24.0;
			let pal_h = 150.0;
			let list_rect = Rect::from_min_max(Pos2::new(rect.min.x + 12.0, y), Pos2::new(rect.max.x - 12.0, rect.max.y - 62.0 - pal_h));
			painter.line_segment([Pos2::new(rect.min.x + 1.0, y), Pos2::new(rect.max.x - 1.0, y)], Stroke::new(1.0, p.div));
			let mut c = ui.new_child(egui::UiBuilder::new().max_rect(list_rect));
			c.set_clip_rect(list_rect);
			let row_h = 40.0;
			let mut pick: Option<u32> = None;
			egui::ScrollArea::vertical().id_salt("btn_list").auto_shrink([false, false]).show_rows(&mut c, row_h, actions.len(), |ui, range| {
				for i in range {
					let (n, cnt, ex) = actions[i];
					let (rr, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
					if self.btn_sel == Some(*n) {
						ui.painter().rect_filled(rr, 6, p.sel);
					} else if resp.hovered() {
						ui.painter().rect_filled(rr, 6, p.div2);
					}
					ui.painter().text(Pos2::new(rr.min.x + 10.0, rr.center().y), Align2::LEFT_CENTER, kb::code_label(*n), f_mono(13.0), p.text);
					ui.painter().text(Pos2::new(rr.min.x + 90.0, rr.center().y), Align2::LEFT_CENTER, lformat!("{cnt} раз", "{cnt}×"), f_body(12.0), p.muted);
					let hint = kb::action_name(*n).unwrap_or("");
					paint_text_clipped(ui, Rect::from_min_size(Pos2::new(rr.min.x + 160.0, rr.min.y + 2.0), Vec2::new(rr.width() - 360.0, 16.0)), hint, f_semi(12.0), p.accent_strong, Align2::LEFT_CENTER);
					paint_text_clipped(ui, Rect::from_min_size(Pos2::new(rr.min.x + 160.0, rr.min.y + 19.0), Vec2::new(rr.width() - 360.0, 18.0)), ex, f_body(12.0), p.text, Align2::LEFT_CENTER);
					let ar = Rect::from_min_max(Pos2::new(rr.max.x - 190.0, rr.min.y + 5.0), Pos2::new(rr.max.x - 8.0, rr.max.y - 5.0));
					match (self.btn.label(*n), self.btn.icon(ctx, *n)) {
						(Some(l), Some((tex, asp))) => {
							let ih = 26.0;
							ui.painter().image(tex.id(), Rect::from_min_size(Pos2::new(ar.max.x - 110.0 - ih * asp, ar.center().y - ih / 2.0), Vec2::new(ih * asp, ih)), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
							ui.painter().text(Pos2::new(ar.max.x - 100.0, ar.center().y - 7.0), Align2::LEFT_CENTER, l, f_body(12.5), p.text);
							ui.painter().text(Pos2::new(ar.max.x - 100.0, ar.center().y + 8.0), Align2::LEFT_CENTER, if self.btn.is_custom(*n) { tr("назначено вами", "assigned by you") } else { tr("из файлов игры", "from the game's files") }, f_body(10.5), p.muted);
						}
						(Some(l), None) => {
							ui.painter().text(Pos2::new(ar.max.x - 100.0, ar.center().y), Align2::LEFT_CENTER, l, f_body(12.5), p.text);
						}
						_ => {
							ui.painter().rect(Rect::from_min_size(Pos2::new(ar.max.x - 110.0, ar.min.y + 2.0), Vec2::new(106.0, ar.height() - 4.0)), 6, Color32::TRANSPARENT, Stroke::new(1.0, p.div), StrokeKind::Inside);
							ui.painter().text(Pos2::new(ar.max.x - 57.0, ar.center().y), Align2::CENTER_CENTER, tr("не назначено", "not assigned"), f_body(12.0), p.muted);
						}
					}
					if resp.clicked() {
						pick = Some(*n);
					}
				}
			});
			if let Some(n) = pick {
				self.btn_sel = Some(n);
			}
			if actions.is_empty() {
				painter.text(list_rect.center(), Align2::CENTER_CENTER, if self.data.is_some() { if ed7 { tr("В русском тексте нет кнопок #160I+", "The Russian text has no #160I+ buttons") } else { tr("В русском тексте нет кодов 900+", "The Russian text has no 900+ codes") } } else { tr("Загрузите файлы — тогда здесь появятся коды из текста", "Load the files and the text's codes will appear here") }, f_body(13.0), p.muted);
			}
			let py = rect.max.y - 58.0 - pal_h;
			painter.line_segment([Pos2::new(rect.min.x + 1.0, py), Pos2::new(rect.max.x - 1.0, py)], Stroke::new(1.0, p.div));
			let sel = self.btn_sel;
			let title = match sel {
				Some(n) => lformat!("Кнопка для {} — {}", "Button for {}: {}", kb::code_label(n), DEVICES[self.btn.device]),
				None => tr("Выберите код в списке", "Choose a code in the list").into(),
			};
			painter.galley(Pos2::new(x0, py + 8.0), ui.fonts(|f| f.layout_job(caps_job(&title, p.muted))), p.muted);
			if self.btn.device < 5 {
				painter.text(Pos2::new(rect.max.x - 22.0, py + 14.0), Align2::RIGHT_CENTER, tr("Ctrl+клик — комбинация (L2+R2) · кнопка по месту — на других геймпадах своя", "Ctrl+click: a combination (L2+R2) · a button by position differs on other gamepads"), f_body(11.5), p.muted);
			} else {
				painter.text(Pos2::new(rect.max.x - 22.0, py + 14.0), Align2::RIGHT_CENTER, tr("Ctrl+клик — комбинация клавиш", "Ctrl+click: a key combination"), f_body(11.5), p.muted);
			}
			let items: Vec<(String, String)> = if self.btn.device < 5 { pad_table().iter().map(|x| (format!("p:{}:{}", self.btn.device, x.0), x.0.to_string())).collect() } else { kb::keys().into_iter().map(|k| (format!("k:{}", k.0), k.0)).collect() };
			let pr = Rect::from_min_max(Pos2::new(x0 - 6.0, py + 26.0), Pos2::new(rect.max.x - 16.0, py + pal_h));
			let mut pc = ui.new_child(egui::UiBuilder::new().max_rect(pr));
			pc.set_clip_rect(pr);
			let cw = if self.btn.device < 5 { 60.0 } else { 46.0 };
			let mut chosen: Option<String> = None;
			egui::ScrollArea::vertical().id_salt("btn_pal").auto_shrink([false, false]).show(&mut pc, |ui| {
				ui.horizontal_wrapped(|ui| {
					ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
					for (key, val) in &items {
						let (r, resp) = ui.allocate_exact_size(Vec2::new(cw, if self.btn.device < 5 { 56.0 } else { 46.0 }), Sense::click());
						let cur = sel.map(|n| if self.btn.device < 5 { self.btn.pad_of(n) } else { self.btn.kb_of(n) }).flatten().map(|c| kb::split_combo(&c).contains(val)).unwrap_or(false);
						if cur {
							ui.painter().rect(r, 7, p.sel, Stroke::new(1.0, p.accent), StrokeKind::Inside);
						} else if resp.hovered() {
							ui.painter().rect_filled(r, 7, p.div2);
						}
						if let Some((tex, asp)) = self.btn.cell(ctx, key) {
							let ih = if self.btn.device < 5 { 30.0f32 } else { 32.0 };
							let iw2 = (ih * asp).min(cw - 6.0);
							let ih2 = iw2 / asp;
							ui.painter().image(tex.id(), Rect::from_center_size(Pos2::new(r.center().x, r.min.y + 4.0 + ih / 2.0), Vec2::new(iw2, ih2)), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
						} else {
							ui.painter().text(Pos2::new(r.center().x, r.min.y + 18.0), Align2::CENTER_CENTER, val, f_body(12.0), p.text);
						}
						if self.btn.device < 5 {
							let lab = pad_table().iter().find(|x| x.0 == val).map(|x| x.1).unwrap_or("");
							paint_text_clipped(ui, Rect::from_min_size(Pos2::new(r.min.x, r.max.y - 16.0), Vec2::new(cw, 14.0)), lab, f_body(10.5), p.muted, Align2::CENTER_CENTER);
						}
						let resp = resp.on_hover_text(val.as_str());
						if resp.clicked() && sel.is_some() {
							let add = ui.input(|i| i.modifiers.ctrl || i.modifiers.shift);
							let n = sel.unwrap();
							let now = if self.btn.device < 5 { self.btn.pad_of(n) } else { self.btn.kb_of(n) };
							chosen = Some(match (add, now) {
								(true, Some(c)) => {
									let mut parts = kb::split_combo(&c);
									if let Some(i) = parts.iter().position(|x| x == val) {
										if parts.len() > 1 {
											parts.remove(i);
										}
									} else {
										parts.push(val.clone());
									}
									parts.join("+")
								}
								_ => val.clone(),
							});
						}
					}
				});
			});
			if let (Some(v), Some(n)) = (chosen, sel) {
				if self.btn.device < 5 {
					self.btn.pad.insert(n, v);
				} else {
					self.btn.kb.insert(n, v);
				}
				self.btn.save_map();
				self.btn.changed();
				changed = true;
			}
			let fr = Rect::from_min_max(Pos2::new(rect.min.x + 22.0, rect.max.y - 50.0), Pos2::new(rect.max.x - 22.0, rect.max.y - 12.0));
			let mut fu = ui.new_child(egui::UiBuilder::new().max_rect(fr).layout(egui::Layout::right_to_left(egui::Align::Center)));
			fu.spacing_mut().item_spacing.x = 8.0;
			if primary_button_icon(&mut fu, &p, tr("Готово", "Done"), Some("check"), Vec2::new(110.0, 34.0), 8, f_semi(14.0), true).clicked() {
				close = true;
			}
			let can_clear = sel.map(|n| if self.btn.device < 5 { self.btn.pad.contains_key(&n) } else { self.btn.kb.contains_key(&n) }).unwrap_or(false);
			if small_button(&mut fu, &p, tr("Убрать своё назначение", "Remove my assignment"), Some("eraser"), 34.0, can_clear, p.text).clicked() {
				if let Some(n) = sel {
					if self.btn.device < 5 {
						self.btn.pad.remove(&n);
					} else {
						self.btn.kb.remove(&n);
					}
					self.btn.save_map();
					self.btn.changed();
					changed = true;
				}
			}
			let mut lu = ui.new_child(egui::UiBuilder::new().max_rect(fr).layout(egui::Layout::left_to_right(egui::Align::Center)));
			lu.label(egui::RichText::new(lformat!("Назначения хранятся в {} рядом с программой.", "Assignments are stored in {} next to the program.", kb::map_file())).font(f_body(11.5)).color(p.muted));
		});
		if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
			close = true;
		}
		if changed {
			self.btn.ensure_loaded();
			self.save_settings();
			self.reset_layouts();
		}
		if close {
			self.btn_open = false;
		}
	}

	fn hidden_window(&mut self, ctx: &egui::Context, all: &[Row]) {
		let p = self.pal;
		let mut list: Vec<(String, String, String, String)> = Vec::new();
		let mut seen = HashSet::new();
		for r in all {
			if self.ignored.contains(&r.key) && seen.insert(r.key.clone()) {
				let sec = checks::cats()[r.cat].0.to_string();
				let wh = if r.pair == checks::NO_PAIR { tr("файл", "file").to_string() } else if r.cat == checks::CAT_UNTR && r.count > 1 { format!("{} ({})", r.files, group(r.count)) } else { lformat!("{} · строка {}", "{} · line {}", r.file, r.line) };
				let t = if r.cat == checks::CAT_UNTR { r.en.clone() } else { r.ru.clone() };
				list.push((r.key.clone(), sec, wh, t));
			}
		}
		for k in &self.ignored {
			if !seen.contains(k) {
				let mut it = k.split('\t');
				let sec = it.next().unwrap_or("").to_string();
				let f = it.next().unwrap_or("").to_string();
				let rest = it.next().unwrap_or("").to_string();
				list.push((k.clone(), sec, if f.is_empty() { "—".into() } else { f }, lformat!("{rest} (сейчас не найдено)", "{rest} (not found now)")));
			}
		}
		list.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
		let mut back: Vec<String> = Vec::new();
		let mut close = false;
		let mut bar_new = self.bar_set.remove("hidden");
		let mut bar_out: Option<f32> = None;
		let screen = ctx.screen_rect();
		egui::Area::new(egui::Id::new("hid_dim")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
			let (r, _) = ui.allocate_exact_size(screen.size(), Sense::click());
			ui.painter().rect_filled(r, 0, Color32::from_black_alpha(if p.dark { 120 } else { 70 }));
		});
		let w = 900f32.min(screen.width() - 40.0);
		let h = 620f32.min(screen.height() - 40.0);
		let rect = Rect::from_center_size(screen.center(), Vec2::new(w, h));
		egui::Area::new(egui::Id::new("hid_box")).order(egui::Order::Foreground).fixed_pos(rect.min).show(ctx, |ui| {
			let _ = ui.allocate_exact_size(rect.size(), Sense::hover());
			let painter = ui.painter().clone();
			painter.add(egui::epaint::Shadow { offset: [0, 14], blur: 40, spread: 0, color: Color32::from_black_alpha(70) }.as_shape(rect, 14));
			painter.rect(rect, 14, p.bg, Stroke::new(1.0, p.div), StrokeKind::Inside);
			painter.text(Pos2::new(rect.min.x + 22.0, rect.min.y + 26.0), Align2::LEFT_CENTER, lformat!("Скрытые — отмечены «Не ошибка» · {}", "Hidden: marked 'Not an error' · {}", group(list.len())), f_semi(18.0), p.text);
			painter.text(Pos2::new(rect.min.x + 22.0, rect.min.y + 50.0), Align2::LEFT_CENTER, tr("«Вернуть» — строка снова появится в своём разделе.", "'Restore' puts the line back in its section."), f_body(13.0), p.muted);
			let body = Rect::from_min_max(Pos2::new(rect.min.x + 12.0, rect.min.y + 70.0), Pos2::new(rect.max.x - 8.0, rect.max.y - 62.0));
			painter.line_segment([Pos2::new(rect.min.x + 1.0, body.min.y), Pos2::new(rect.max.x - 1.0, body.min.y)], Stroke::new(1.0, p.div));
			let lr = Rect::from_min_max(body.min, Pos2::new(body.max.x - BAR_W, body.max.y));
			let mut c = ui.new_child(egui::UiBuilder::new().max_rect(lr));
			c.set_clip_rect(lr);
			let row_h = 48.0;
			let mut sa = egui::ScrollArea::vertical().id_salt("hidden_list").auto_shrink([false, false]).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
			if let Some(o) = bar_new.take() {
				sa = sa.vertical_scroll_offset(o);
			}
			let out = sa.show_rows(&mut c, row_h, list.len(), |ui, range| {
				for i in range {
					let (key, sec, wh, t) = &list[i];
					let (rr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::hover());
					let sg = ui.painter().layout_no_wrap(sec.clone(), f_body(12.0), p.text);
					let sr = Rect::from_min_size(Pos2::new(rr.min.x + 6.0, rr.center().y - 11.0), Vec2::new(sg.size().x + 14.0, 22.0));
					ui.painter().rect_filled(sr, 4, p.sel);
					ui.painter().galley(Pos2::new(sr.min.x + 7.0, sr.center().y - sg.size().y / 2.0), sg, p.text);
					paint_text_clipped(ui, Rect::from_min_size(Pos2::new(rr.min.x + 150.0, rr.min.y + 6.0), Vec2::new(rr.width() - 270.0, 18.0)), wh, f_mono(12.0), p.muted, Align2::LEFT_CENTER);
					paint_text_clipped(ui, Rect::from_min_size(Pos2::new(rr.min.x + 150.0, rr.min.y + 25.0), Vec2::new(rr.width() - 270.0, 18.0)), t, f_body(12.5), p.text, Align2::LEFT_CENTER);
					let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(Pos2::new(rr.max.x - 110.0, rr.min.y + 8.0), Pos2::new(rr.max.x - 4.0, rr.max.y - 8.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
					if small_button(&mut bu, &p, tr("Вернуть", "Restore"), Some("rotate-ccw"), 30.0, true, p.accent).clicked() {
						back.push(key.clone());
					}
					ui.painter().line_segment([Pos2::new(rr.min.x, rr.max.y), Pos2::new(rr.max.x, rr.max.y)], Stroke::new(1.0, p.div2));
				}
			});
			if let Some(o) = vbar(ui, &p, "hidden", Rect::from_min_max(Pos2::new(body.max.x - BAR_W + 2.0, body.min.y + 4.0), Pos2::new(body.max.x - 3.0, body.max.y - 4.0)), out.state.offset.y, out.content_size.y, out.inner_rect.height()) {
				bar_out = Some(o);
			}
			if list.is_empty() {
				painter.text(body.center(), Align2::CENTER_CENTER, tr("Скрытых строк нет", "No hidden lines"), f_body(14.0), p.muted);
			}
			let fr = Rect::from_min_max(Pos2::new(rect.min.x + 22.0, rect.max.y - 54.0), Pos2::new(rect.max.x - 22.0, rect.max.y - 12.0));
			let mut fu = ui.new_child(egui::UiBuilder::new().max_rect(fr).layout(egui::Layout::right_to_left(egui::Align::Center)));
			fu.spacing_mut().item_spacing.x = 8.0;
			if primary_button_icon(&mut fu, &p, tr("Закрыть", "Close"), Some("x"), Vec2::new(110.0, 34.0), 8, f_semi(14.0), true).clicked() {
				close = true;
			}
			if small_button(&mut fu, &p, tr("Вернуть все", "Restore all"), Some("rotate-ccw"), 34.0, !list.is_empty(), p.text).clicked() {
				back = list.iter().map(|x| x.0.clone()).collect();
			}
		});
		if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
			close = true;
		}
		if !back.is_empty() {
			for k in back {
				self.ignored.remove(&k);
			}
			self.rows_cache = None;
			write_set(&ign_file(), tr("скрытые кнопкой «Не ошибка» (раздел, файл, строка)", "hidden with the 'Not an error' button (section, file, line)"), &self.ignored);
		}
		if let Some(o) = bar_out {
			self.bar_set.insert("hidden", o);
			ctx.request_repaint();
		}
		if close {
			self.hidden_open = false;
		}
	}
}

const BAR_W: f32 = 14.0;

fn vbar(ui: &egui::Ui, p: &Pal, id: &'static str, track: Rect, offset: f32, content: f32, view: f32) -> Option<f32> {
	if content <= view + 1.0 || track.height() < 20.0 {
		return None;
	}
	let max_off = content - view;
	let hh = (view / content * track.height()).max(64.0).min(track.height());
	let free = (track.height() - hh).max(1.0);
	let t = (offset / max_off).clamp(0.0, 1.0);
	let handle = Rect::from_min_size(Pos2::new(track.min.x, track.min.y + t * free), Vec2::new(track.width(), hh));
	let resp = ui.interact(track, ui.id().with(("vbar", id)), Sense::click_and_drag());
	let grab_id = ui.id().with(("vbar_grab", id));
	let mut new = None;
	if resp.drag_started() || resp.clicked() {
		if let Some(pp) = resp.interact_pointer_pos() {
			let grab = if handle.contains(pp) { pp.y - handle.min.y } else { hh / 2.0 };
			ui.ctx().data_mut(|d| d.insert_temp(grab_id, grab));
			let y = (pp.y - grab - track.min.y).clamp(0.0, free);
			new = Some(y / free * max_off);
		}
	} else if resp.dragged() {
		if let Some(pp) = resp.interact_pointer_pos() {
			let grab: f32 = ui.ctx().data(|d| d.get_temp(grab_id)).unwrap_or(hh / 2.0);
			let y = (pp.y - grab - track.min.y).clamp(0.0, free);
			new = Some(y / free * max_off);
		}
	}
	let hot = resp.hovered() || resp.dragged();
	let painter = ui.painter();
	painter.rect(track, 6, p.div2.gamma_multiply(0.6), Stroke::new(1.0, p.div), StrokeKind::Inside);
	let fill = if resp.dragged() { p.accent_strong } else if hot { p.accent_strong.gamma_multiply(0.9) } else { p.accent };
	painter.rect(handle.shrink(1.0), 5, fill, Stroke::new(1.0, p.accent_strong), StrokeKind::Inside);
	if hot {
		ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
	}
	new
}
