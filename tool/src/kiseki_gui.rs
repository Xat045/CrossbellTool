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
use crate::kai_pick::{Event as PEvent, Item as PItem, Mode as PMode, Picker};
use crossbell_tool::jobs::{self, Options};
use crossbell_tool::scena::{detect_title, Title as Game};

const TITLE: &str = concat!("CrossbellTool ", env!("VER_CROSSBELLTOOL"), env!("VER_SUFFIX"), " — Trails from Zero / Trails to Azure");
fn game_items() -> [&'static str; 3] {
	[tr("Определить по папке игры", "Auto-detect"), "Trails from Zero", "Trails to Azure"]
}

#[derive(Default)]
struct Shared {
	log: Mutex<Vec<String>>,
	done: AtomicUsize,
	total: AtomicUsize,
	finished: AtomicBool,
	cancel: AtomicBool,
	current: Mutex<String>,
	check: Mutex<Vec<String>>,
}

struct GuiSink {
	shared: Arc<Shared>,
	ctx: egui::Context,
}

fn log_path() -> PathBuf {
	exe_dir().join("CrossbellTool.log")
}

impl jobs::Sink for GuiSink {
	fn log(&self, s: &str) {
		self.shared.log.lock().unwrap().push(s.to_string());
		use std::io::Write;
		if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_path()) {
			let _ = f.write_all(format!("{s}\r\n").as_bytes());
		}
		self.ctx.request_repaint();
	}
	fn progress(&self, done: usize, total: usize) {
		self.shared.done.store(done, Ordering::SeqCst);
		self.shared.total.store(total, Ordering::SeqCst);
		self.ctx.request_repaint();
	}
	fn cancelled(&self) -> bool {
		self.shared.cancel.load(Ordering::SeqCst)
	}
	fn current(&self, name: &str) {
		*self.shared.current.lock().unwrap() = name.to_string();
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
	Decompile,
	Compile,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
	Start,
	Normal,
	Done,
	Warn,
}

struct LogLine {
	time: String,
	text: String,
	kind: Kind,
}

#[derive(Clone)]
enum Action {
	Start(Op),
	ForceBuild,
}

fn sel_game(sel: usize) -> Option<Game> {
	match sel {
		1 => Some(Game::Zero),
		2 => Some(Game::Azure),
		_ => None,
	}
}


trait GameName {
	fn name(self) -> &'static str;
}
impl GameName for Game {
	fn name(self) -> &'static str {
		self.label()
	}
}

fn data_dir(src: &str) -> PathBuf {
	let p = PathBuf::from(src.trim().trim_matches('"'));
	if p.join("data").is_dir() {
		p.join("data")
	} else {
		p
	}
}

fn now() -> String {
	let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64 + local_offset();
	let s = t.rem_euclid(86400);
	format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

fn local_offset() -> i64 {
	#[cfg(windows)]
	{
		#[repr(C)]
		struct SystemTime {
			y: u16, m: u16, dow: u16, d: u16, h: u16, mi: u16, s: u16, ms: u16,
		}
		#[repr(C)]
		struct Tzi {
			bias: i32,
			sname: [u16; 32],
			sdate: SystemTime,
			sbias: i32,
			dname: [u16; 32],
			ddate: SystemTime,
			dbias: i32,
		}
		unsafe extern "system" {
			fn GetTimeZoneInformation(tz: *mut Tzi) -> u32;
		}
		unsafe {
			let mut tz: Tzi = std::mem::zeroed();
			let r = GetTimeZoneInformation(&mut tz);
			let extra = if r == 2 { tz.dbias } else { tz.sbias };
			-((tz.bias + extra) as i64) * 60
		}
	}
	#[cfg(not(windows))]
	{
		0
	}
}

struct App {
	pal: Pal,
	settings: Settings,
	src: String,
	proj: String,
	out: String,
	mask: String,
	game_sel: usize,
	mode: usize,
	tab: usize,
	log: Vec<LogLine>,
	log_to_end: bool,
	shared: Option<Arc<Shared>>,
	op: Option<Op>,
	op_start: Instant,
	last_done: Option<Op>,
	status: String,
	flash: Option<(String, Instant)>,
	dialog: Dialog<Action>,
	first_frame: bool,
	pick: Option<Picker>,
	dec_sel: Option<(String, Vec<PItem>)>,
	build_est: u64,
	build_once: Option<Vec<String>>,
	tex_dds: bool,
	skip_check: bool,
	last_build: Option<Vec<String>>,
	build_ready: bool,
	det_cache: std::cell::RefCell<Option<(String, Option<Game>)>>,
}

impl App {
	fn new(cc: &eframe::CreationContext) -> App {
		setup_fonts(&cc.egui_ctx);
		set_dialog_owner(cc);
		egui_extras::install_image_loaders(&cc.egui_ctx);
		for old in ["Azure-Zero.ini", "KisekiTool.ini"] {
			if !exe_dir().join("CrossbellTool.ini").exists() && exe_dir().join(old).exists() {
				let _ = std::fs::copy(exe_dir().join(old), exe_dir().join("CrossbellTool.ini"));
			}
		}
		let st = Settings::load("CrossbellTool.ini", "CrossbellTool");
		UI_EN.store(st.get("lang") != Some("ru"), std::sync::atomic::Ordering::Relaxed);
		let dark = st.get("dark") == Some("1");
		let pal = Pal::get(dark);
		apply_visuals(&cc.egui_ctx, &pal);
		let game_sel = st.get("title").and_then(|v| v.parse::<usize>().ok()).filter(|&v| v < 3).unwrap_or(0);
		let g = sel_game(game_sel);
		let key_p = g.map(|g| format!("proj_{}", g.key())).unwrap_or_else(|| "proj".into());
		let key_o = g.map(|g| format!("out_{}", g.key())).unwrap_or_else(|| "out".into());
		let proj = st.get(&key_p).or(st.get("proj")).map(|s| s.to_string()).unwrap_or_default();
		let out = st.get(&key_o).or(st.get("out")).map(|s| s.to_string()).unwrap_or_default();
		crate::kai_pick::set_speed(st.get("unpack_speed").and_then(|v| v.parse().ok()).unwrap_or(0.0));
		crate::kai_pick::set_build_speed(st.get("build_speed").and_then(|v| v.parse().ok()).unwrap_or(0.0));
		let app = App {
			pal,
			src: st.get_or("game", "").to_string(),
			proj,
			out,
			mask: st.get_or("mask", "").to_string(),
			game_sel,
			mode: if st.get("what") == Some("all") { 1 } else { 0 },
			tab: 0,
			log: Vec::new(),
			log_to_end: false,
			shared: None,
			op: None,
			op_start: Instant::now(),
			last_done: None,
			status: tr("Готов к работе", "Ready").into(),
			flash: None,
			dialog: Dialog::None,
			first_frame: true,
			pick: None,
			dec_sel: None,
			build_est: 0,
			build_once: None,
			tex_dds: st.get("tex_fmt") == Some("dds"),
			skip_check: false,
			last_build: None,
			build_ready: false,
			det_cache: Default::default(),
			settings: st,
		};
		app
	}

	fn save_settings(&mut self) {
		let st = &mut self.settings;
		st.set("game", &self.src);
		st.set("mask", &self.mask);
		st.set("tex_fmt", if self.tex_dds { "dds" } else { "png" });
		st.set("what", if self.mode == 1 { "all" } else { "text" });
		st.set("title", &self.game_sel.to_string());
		st.set("dark", if self.pal.dark { "1" } else { "0" });
		st.set("lang", if ui_en() { "en" } else { "ru" });
		match sel_game(self.game_sel) {
			Some(g) => {
				st.set(&format!("proj_{}", g.key()), &self.proj);
				st.set(&format!("out_{}", g.key()), &self.out);
			}
			None => {
				st.set("proj", &self.proj);
				st.set("out", &self.out);
			}
		}
		st.set("unpack_speed", &format!("{:.0}", crate::kai_pick::speed()));
		st.set("build_speed", &format!("{:.0}", crate::kai_pick::build_speed()));
		st.save();
	}

	fn on_game_changed(&mut self, old: usize) {
		let new = self.game_sel;
		{
			let st = &mut self.settings;
			match sel_game(old) {
				Some(g) => {
					st.set(&format!("proj_{}", g.key()), &self.proj);
					st.set(&format!("out_{}", g.key()), &self.out);
				}
				None => {
					st.set("proj", &self.proj);
					st.set("out", &self.out);
				}
			}
		}
		let g = sel_game(new);
		let (kp, ko) = match g {
			Some(g) => (format!("proj_{}", g.key()), format!("out_{}", g.key())),
			None => ("proj".into(), "out".into()),
		};
		let st = &self.settings;
		self.proj = st.get(&kp).map(|s| s.to_string()).unwrap_or_else(|| self.proj.clone());
		self.out = st.get(&ko).map(|s| s.to_string()).unwrap_or_else(|| self.out.clone());
		self.save_settings();
	}

	fn title(&self) -> Option<Game> {
		sel_game(self.game_sel).or_else(|| self.detected())
	}

	fn options(&self) -> Options {
		Options {
			title: self.title().unwrap_or(Game::Zero),
			game_dir: data_dir(&self.src),
			proj_dir: PathBuf::from(self.proj.trim()),
			out_dir: PathBuf::from(self.out.trim()),
			scena: true,
			tables: true,
			textures: true,
			tex_dds: self.tex_dds,
			check: !self.skip_check,
			mask: self.mask.trim().to_string(),
			only_changed: true,
			overwrite_edited: false,
			include: self.sel_items().map(|v| v.iter().filter(|i| i.on).map(|i| i.key.clone()).collect()),
			build_only: self.build_once.clone(),
		}
	}

	fn src_dir(&self) -> Option<String> {
		if self.src.trim().is_empty() || self.src.contains(';') {
			return None;
		}
		let d = data_dir(&self.src);
		d.is_dir().then(|| d.display().to_string())
	}

	fn sel_items(&self) -> Option<&Vec<PItem>> {
		let d = self.src_dir()?;
		match &self.dec_sel {
			Some((dir, items)) if *dir == d => Some(items),
			_ => None,
		}
	}

	fn open_pick(&mut self, then_start: bool, ctx: &egui::Context) {
		let Some(d) = self.src_dir() else { return };
		let mut pk = match self.sel_items() {
			Some(items) => Picker::with_items_ed7(PMode::Decompile, &d, items.clone(), then_start),
			None => Picker::open_ed7(PMode::Decompile, &d, then_start, ctx),
		};
		pk.tex_dds = self.tex_dds;
		self.pick = Some(pk);
	}

	fn detected(&self) -> Option<Game> {
		if self.src.trim().is_empty() {
			return None;
		}
		detect_title(&PathBuf::from(self.src.trim().trim_matches('"')))
	}

	fn detected_ui(&self) -> Option<Game> {
		if let Some((src, g)) = self.det_cache.borrow().as_ref() {
			if *src == self.src {
				return *g;
			}
		}
		let g = self.detected();
		*self.det_cache.borrow_mut() = Some((self.src.clone(), g));
		g
	}

	fn push_log(&mut self, text: &str, kind: Kind) {
		self.log.push(LogLine { time: now(), text: text.to_string(), kind });
		self.log_to_end = true;
	}

	fn classify(s: &str) -> Kind {
		let t = s.trim_start();
		if t.starts_with(tr("Готово", "Done")) {
			Kind::Done
		} else if t.starts_with('✖') || t.starts_with(tr("Остановлено", "Stopped")) || t.starts_with("⚠") || t.contains(tr("ошибка", "error")) || t.contains(tr("Ошибка", "Error")) {
			Kind::Warn
		} else if t.starts_with(tr("Декомпиляция (", "Decompiling (")) || t.starts_with(tr("Игра:", "Game:")) || t.starts_with(tr("Сборка", "Building")) && !t.starts_with(tr("Сборка завершена", "Build finished")) {
			Kind::Start
		} else {
			Kind::Normal
		}
	}

	fn start(&mut self, op: Op, ctx: &egui::Context, confirmed: bool) {
		if self.shared.is_some() {
			return;
		}
		let compile = op == Op::Compile;
		if !compile && !confirmed && self.src_dir().is_some() && self.sel_items().is_none() {
			self.open_pick(true, ctx);
			return;
		}
		if compile && !confirmed && !self.build_ready {
			let proj = self.proj.trim().to_string();
			if !proj.is_empty() && std::path::Path::new(&proj).is_dir() {
				self.pick = Some(Picker::open_ed7(PMode::Build, &proj, true, ctx));
				return;
			}
		}
		let o = self.options();
		if !compile && !o.game_dir.is_dir() {
			self.tab = 1;
			self.push_log(tr("Укажите папку игры или файлы.", "Choose the game folder or files."), Kind::Warn);
			return;
		}
		if !compile && self.title().is_none() && !confirmed {
			self.dialog = Dialog::Ask(tr("Не удалось определить игру по папке игры.\nБудет Trails from Zero. Если это Trails to Azure — выберите игру в списке «Игра».\n\nПродолжить?", "Could not detect the game from the game folder.\nTrails from Zero will be used. If this is Trails to Azure, choose it in the 'Game' list.\n\nContinue?").into(), Action::Start(op));
			return;
		}
		if o.proj_dir.as_os_str().is_empty() || (compile && o.out_dir.as_os_str().is_empty()) {
			self.dialog = Dialog::Info(tr("Укажите папку для правки и папку для собранного.", "Choose the project folder and the output folder.").into());
			return;
		}
		if !confirmed {
			let (need, dir, what) = if compile {
				(self.build_est, &o.out_dir, tr("собранного", "output"))
			} else {
				(self.sel_items().map(|v| v.iter().filter(|i| i.on).map(|i| i.unpacked).sum()).unwrap_or(0), &o.proj_dir, tr("для правки", "project"))
			};
			let free = crossbell_tool::kai::jobs::free_space(dir);
			if let Some(f) = free.filter(|&f| need > 0 && f < need + (need >> 5)) {
				let gb = |b: u64| b as f64 / 1073741824.0;
				self.dialog = Dialog::Ask(
					lformat!("НЕ ХВАТИТ МЕСТА НА ДИСКЕ.\n\n{} примерно {:.2} ГБ, а на диске папки {what} свободно {:.2} ГБ.\n\nВыберите папку на диске, где больше места, или освободите место.\n\nВсё равно начать?", "NOT ENOUGH DISK SPACE.\n\n{} about {:.2} GB, but the {what} folder's disk has {:.2} GB free.\n\nChoose a folder on a disk with more space, or free some space.\n\nStart anyway?",
						if compile { tr("Собранные файлы займут", "The built files will take") } else { tr("Разобранные файлы займут", "The unpacked files will take") },
						gb(need),
						gb(f)
					),
					Action::Start(op),
				);
				return;
			}
		}
		let into_game = o.out_dir.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("data")).unwrap_or(false)
			|| (!self.src.trim().is_empty() && same_dir(&o.out_dir, &o.game_dir));
		if compile && !confirmed && into_game {
			self.dialog = Dialog::Ask(lformat!("Собранные файлы будут записаны прямо в {}\nи заменят файлы там. Сделайте резервную копию, если её нет.\n\nПродолжить?", "The built files will be written directly to {}\nand replace the files there. Make a backup if you don't have one.\n\nContinue?", o.out_dir.display()), Action::Start(op));
			return;
		}
		self.save_settings();
		self.log.clear();
		let _ = std::fs::write(log_path(), "\u{FEFF}");
		self.tab = 1;
		self.status = if compile { tr("Сборка…", "Building…").into() } else { tr("Декомпиляция…", "Decompiling…").into() };
		let shared = Arc::new(Shared::default());
		self.shared = Some(shared.clone());
		self.op = Some(op);
		self.op_start = Instant::now();
		self.last_build = o.build_only.clone();
		self.skip_check = false;
		self.build_once = None;
		self.build_ready = false;
		let est: u64 = if compile { self.build_est } else { self.sel_items().map(|v| v.iter().filter(|i| i.on).map(|i| i.unpacked).sum()).unwrap_or(0) };
		let sink = GuiSink { shared, ctx: ctx.clone() };
		std::thread::spawn(move || {
			let start = Instant::now();
			let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| if compile { jobs::compile(&o, &sink) } else { jobs::decompile(&o, &sink) }));
			if res.is_err() {
				jobs::Sink::log(&sink, tr("✖ Внутренняя ошибка программы.", "✖ Internal program error."));
			}
			let cancelled = sink.shared.cancel.load(Ordering::SeqCst);
			if cancelled {
				jobs::Sink::log(&sink, tr("Остановлено пользователем.", "Stopped by the user."));
			}
			jobs::Sink::log(&sink, &lformat!("Время: {:.1} с", "Time: {:.1} s", start.elapsed().as_secs_f32()));
			if let (true, false, Ok(s)) = (compile, cancelled, &res) {
				let secs = start.elapsed().as_secs_f64();
				if est > 0 && secs >= 10.0 && s.ok > 0 {
					crate::kai_pick::set_build_speed(est as f64 / secs);
				}
			}
			if let (false, false, Ok(s)) = (compile, cancelled, &res) {
				let secs = start.elapsed().as_secs_f64();
				if est > 0 && secs >= 30.0 && s.ok > 0 {
					crate::kai_pick::set_speed(est as f64 / secs);
				}
			}
			if !cancelled && start.elapsed().as_secs() >= 10 {
				let what = if compile { tr("сборка", "build") } else { tr("декомпиляция", "decompile") };
				let text = match &res {
					Ok(s) => lformat!("Готово: успешно {}, ошибок {}{}", "Done: {} ok, {} errors{}", s.ok, s.failed, if s.warnings > 0 { lformat!(", предупреждений {}", ", {} warnings", s.warnings) } else { String::new() }),
					Err(_) => tr("Внутренняя ошибка программы — см. журнал", "Internal program error, see the log").into(),
				};
				crate::gui_common::notify_done(&lformat!("CrossbellTool: {what} завершена", "CrossbellTool: {what} finished"), &text);
			}
			if let Ok(s) = &res {
				*sink.shared.check.lock().unwrap() = s.check.clone();
			}
			sink.shared.finished.store(true, Ordering::SeqCst);
			sink.ctx.request_repaint();
		});
	}

	fn poll(&mut self) {
		let Some(sh) = self.shared.clone() else { return };
		let lines: Vec<String> = std::mem::take(&mut *sh.log.lock().unwrap());
		for l in lines {
			for part in l.lines() {
				let k = Self::classify(part);
				self.push_log(part, k);
			}
		}
		let (d, t) = (sh.done.load(Ordering::SeqCst), sh.total.load(Ordering::SeqCst));
		if t > 0 && !sh.finished.load(Ordering::SeqCst) {
			let what = if self.op == Some(Op::Compile) { tr("Сборка", "Building") } else { tr("Декомпиляция", "Decompiling") };
			let left = if d >= 3 && d < t { lformat!(" · осталось {}", " · {} left", crate::kai_pick::fmt_time(self.op_start.elapsed().as_secs_f64() / d as f64 * (t - d) as f64)) } else { String::new() };
			let cur = sh.current.lock().unwrap().clone();
			self.status = lformat!("{what}: {d} из {t} файлов{left}{}", "{what}: {d} of {t} files{left}{}", if cur.is_empty() { String::new() } else { format!(" · {cur}") });
		}
		if sh.finished.load(Ordering::SeqCst) {
			let cancelled = sh.cancel.load(Ordering::SeqCst);
			let check = std::mem::take(&mut *sh.check.lock().unwrap());
			if !check.is_empty() {
				let mut list: Vec<String> = check.iter().take(12).map(|s| format!("· {s}")).collect();
				if check.len() > 12 {
					list.push(lformat!("… и ещё {} (все — в журнале)", "… and {} more (all of them are in the log)", check.len() - 12));
				}
				self.dialog = Dialog::Ask(lformat!("В переводе сломаны служебные коды или подстановки — {}. Игра может показать не то или упасть.\n\n{}\n\nИсправьте их (подробно — вкладка «Проверки» Сверки) или соберите всё равно.\n\nСобрать всё равно?", "Service codes or placeholders are broken in the translation: {}. The game may show the wrong text or crash.\n\n{}\n\nFix them (details are in the Checks tab of CrossbellCompare) or build anyway.\n\nBuild anyway?", check.len(), list.join("\n")), Action::ForceBuild);
			}
			self.status = if cancelled {
				tr("Остановлено", "Stopped").into()
			} else if self.op == Some(Op::Compile) {
				tr("Сборка завершена", "Build finished").into()
			} else {
				tr("Декомпиляция завершена — правьте файлы в папке проекта", "Decompile finished: edit the files in the project folder").into()
			};
			if !cancelled {
				self.last_done = self.op;
			}
			self.shared = None;
			self.op = None;
			self.save_settings();
		}
	}

	fn progress(&self) -> Option<f32> {
		let sh = self.shared.as_ref()?;
		let (d, t) = (sh.done.load(Ordering::SeqCst), sh.total.load(Ordering::SeqCst));
		Some(if t == 0 { 0.0 } else { d as f32 / t as f32 })
	}

	fn flash(&mut self, s: &str) {
		self.flash = Some((s.to_string(), Instant::now()));
	}

	fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		let p = self.pal;
		let full = ui.max_rect();
		let x0 = full.min.x + 22.0;
		let x1 = full.max.x - 22.0;
		let w = x1 - x0;
		let busy = self.shared.is_some();
		let mut y = full.min.y + 18.0;

		let label_h = 16.0;
		ui.painter().galley(Pos2::new(x0, y), ui.fonts(|f| f.layout_job(caps_job(tr("Папка игры или файлы", "Game folder or files"), p.muted))), p.muted);
		let det = self.detected_ui();
		let (right, rc) = if let Some(g) = sel_game(self.game_sel) {
			(lformat!("Выбрано вручную: {}", "Chosen manually: {}", g.name()), p.accent)
		} else if let Some(g) = det {
			(lformat!("Определено по папке: {}", "Detected from the folder: {}", g.name()), p.accent)
		} else if !self.src.trim().is_empty() {
			(tr("Игра не определена — выберите в списке «Игра»", "Game not detected: choose it in the 'Game' list").to_string(), p.warn)
		} else {
			(tr("Игра определится по папке: zero.exe — Zero, ao.exe — Azure", "The game is detected from the folder: zero.exe is Zero, ao.exe is Azure").to_string(), p.muted)
		};
		ui.painter().text(Pos2::new(x1, y + 6.0), Align2::RIGHT_CENTER, right, f_body(11.5), rc);
		y += label_h + 5.0;
		let empty = self.src.trim().is_empty();
		let files_btn = self.src_dir().map(|_| match self.sel_items() {
			Some(v) => lformat!("Файлы: {} из {}", "Files: {} of {}", v.iter().filter(|i| i.on).count(), v.len()),
			None => tr("Файлы…", "Files…").to_string(),
		});
		let fb_w = files_btn.as_ref().map(|t| ui.fonts(|f| f.layout_no_wrap(t.clone(), f_body(13.0), p.text).size().x) + 24.0 + 20.0 + 10.0).unwrap_or(0.0);
		if let Some(t) = &files_btn {
			let br = Rect::from_min_size(Pos2::new(x1 - fb_w + 10.0, y), Vec2::new(fb_w - 10.0, 38.0));
			let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(br));
			if small_button(&mut bu, &p, t, Some("check"), 38.0, !busy, p.accent).on_hover_text(tr("Выбрать галочками, что разбирать", "Tick what to decompile")).clicked() {
				self.open_pick(false, ctx);
			}
		}
		let mut r = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w - fb_w, 38.0))));
		let (_, browse, brect) = path_field(&mut r, &p, "src", &mut self.src, 38.0, tr("Перетащите сюда папку игры — или нажмите «Обзор…»", "Drop the game folder here, or click 'Browse…'"), tr("Обзор…", "Browse…"), empty, !busy);
		let _ = brect;
		if browse {
			if let Some(d) = pick_folder(self.src.trim()) {
				self.src = d.display().to_string();
				self.save_settings();
				self.dec_sel = None;
				self.open_pick(false, ctx);
			}
		}
		y += 38.0 + 12.0;

		let colw = (w - 14.0) / 2.0;
		for (i, (lbl, key)) in [(tr("Куда сохранять для правки", "Project folder (for editing)"), "proj"), (tr("Куда сохранять собранное", "Output folder"), "out")].iter().enumerate() {
			let cx = x0 + i as f32 * (colw + 14.0);
			ui.painter().galley(Pos2::new(cx, y), ui.fonts(|f| f.layout_job(caps_job(lbl, p.muted))), p.muted);
			let mut r = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::new(cx, y + label_h + 5.0), Vec2::new(colw, 36.0))));
			let val = if *key == "proj" { &mut self.proj } else { &mut self.out };
			let (_, clicked, _) = path_field(&mut r, &p, key, val, 36.0, "", tr("Обзор…", "Browse…"), false, !busy);
			if clicked {
				if let Some(d) = pick_folder(val) {
					*val = d.display().to_string();
					self.save_settings();
				}
			}
		}
		y += label_h + 5.0 + 36.0 + 12.0;

		let c1 = 260.0;
		let c3 = w - c1 - 14.0;
		let c2 = -14.0;
		ui.painter().galley(Pos2::new(x0, y), ui.fonts(|f| f.layout_job(caps_job(tr("Игра", "Game"), p.muted))), p.muted);
		ui.painter().galley(Pos2::new(x0 + c1 + c2 + 28.0, y), ui.fonts(|f| f.layout_job(caps_job(tr("Маска файлов", "File mask"), p.muted))), p.muted);
		let fy = y + label_h + 5.0;
		{
			let mut r = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::new(x0, fy), Vec2::new(c1, 36.0))));
			let items: Vec<String> = game_items().iter().map(|s| s.to_string()).collect();
			let old = self.game_sel;
			if combo(&mut r, &p, "game", &items, &mut self.game_sel, c1, 36.0, 8, f_body(13.5)) {
				self.on_game_changed(old);
				if let Some(g) = sel_game(self.game_sel) {
					self.flash(&lformat!("Папки проекта и сборки — для {}", "Project and output folders: for {}", g.name()));
				}
			}
		}
		{
			let mr = Rect::from_min_size(Pos2::new(x0 + c1 + c2 + 28.0, fy), Vec2::new(c3, 36.0));
			let mut r = ui.new_child(egui::UiBuilder::new().max_rect(mr));
			r.painter().rect(mr, 8, p.surface, Stroke::new(1.0, p.div), StrokeKind::Inside);
			let mut cr = r.new_child(egui::UiBuilder::new().max_rect(mr.shrink2(Vec2::new(12.0, 0.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
			let resp = cr
				.add(egui::TextEdit::singleline(&mut self.mask).frame(false).font(f_body(13.0)).desired_width(c3 - 24.0).interactive(!busy).hint_text(tr("все файлы", "all files")).margin(egui::Margin::ZERO))
				.on_hover_text(tr("Только эти файлы, через запятую: c0100, t_town, c01*\nПусто — все файлы", "Only these files, comma-separated: c0100, t_town, c01*\nEmpty means all files"));
			if resp.lost_focus() {
				self.save_settings();
			}
		}
		y = fy + 36.0 + 18.0;

		let bw = (w - 14.0) / 2.0;
		let bh = 76.0;
		let prog = self.progress();
		let dec_part = self.sel_items().and_then(|v| {
			let n = v.iter().filter(|i| i.on).count();
			(n < v.len()).then_some((n, v.len()))
		});
		for (i, op) in [Op::Decompile, Op::Compile].iter().enumerate() {
			let rect = Rect::from_min_size(Pos2::new(x0 + i as f32 * (bw + 14.0), y), Vec2::new(bw, bh));
			let resp = ui.interact(rect, ui.id().with(("main", i)), if busy { Sense::hover() } else { Sense::click() });
			let primary = *op == Op::Decompile;
			let running = self.op == Some(*op);
			let hov = resp.hovered() && !busy;
			let painter = ui.painter().with_clip_rect(rect.expand(1.0));
			if primary {
				painter.rect_filled(rect, 12, if hov { p.accent_strong } else { p.accent });
			} else {
				painter.rect(rect, 12, if hov { p.div2 } else { Color32::TRANSPARENT }, Stroke::new(1.0, p.accent), StrokeKind::Inside);
			}
			let (fg, sub) = if primary { (p.on_accent, p.on_accent.gamma_multiply(0.85)) } else { (p.text, p.muted) };
			let ic = if primary { p.on_accent } else { p.accent };
			let ix = rect.min.x + 20.0;
			paint_icon(ui, if primary { "archive" } else { "package" }, Rect::from_min_size(Pos2::new(ix, rect.center().y - 13.0), Vec2::splat(26.0)), ic);
			let title = match (op, running) {
				(Op::Decompile, true) => tr("Декомпиляция…", "Decompiling…"),
				(Op::Decompile, false) => if dec_part.is_some() { tr("Декомпилировать выбранное", "Decompile selected") } else { tr("Декомпилировать всё", "Decompile all") },
				(Op::Compile, true) => tr("Сборка…", "Building…"),
				(Op::Compile, false) => tr("Собрать", "Build"),
			};
			let subt = if primary {
				match dec_part {
					Some((n, m)) => lformat!("{n} из {m} → папка проекта (.json, .xlsx, .png)", "{n} of {m} → project folder (.json, .xlsx, .png)"),
					None => tr("папка игры → папка для правки", "game folder → project folder").to_string(),
				}
			} else {
				tr("Проект → изменённые файлы игры (.bin, ._dt, .itp)", "Project → changed game files (.bin, ._dt, .itp)").to_string()
			};
			let tx = ix + 26.0 + 14.0;
			painter.text(Pos2::new(tx, rect.center().y - 10.0), Align2::LEFT_CENTER, title, f_semi(21.0), fg);
			painter.text(Pos2::new(tx, rect.center().y + 13.0), Align2::LEFT_CENTER, subt, f_body(13.0), sub);
			if running {
				if let Some(pr) = prog {
					let br = Rect::from_min_size(Pos2::new(rect.min.x, rect.max.y - 4.0), Vec2::new(rect.width() * pr.clamp(0.0, 1.0), 4.0));
					painter.with_clip_rect(rect.shrink(0.5)).rect_filled(br, egui::CornerRadius { nw: 0, ne: 0, sw: 12, se: if pr >= 0.99 { 12 } else { 0 } }, p.ok_strong);
				}
			}
			if resp.clicked() {
				self.start(*op, ctx, false);
			}
			if !busy {
				resp.on_hover_cursor(egui::CursorIcon::PointingHand);
			}
		}
		y += bh + 14.0;

		{
			let row = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 32.0));
			let mut r = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::right_to_left(egui::Align::Center)));
			r.spacing_mut().item_spacing.x = 10.0;
			if small_button(&mut r, &p, tr("Открыть собранное", "Open output"), Some("folder"), 32.0, true, p.text).clicked() {
				open_folder(&self.out);
				self.flash(tr("Открыта папка собранного", "Output folder opened"));
			}
			if small_button(&mut r, &p, tr("Открыть проект", "Open project"), Some("folder"), 32.0, true, p.text).clicked() {
				open_folder(&self.proj);
				self.flash(tr("Открыта папка проекта", "Project folder opened"));
			}
			if small_button(&mut r, &p, tr("Стоп", "Stop"), Some("stop"), 32.0, busy, p.warn).clicked() {
				if let Some(s) = &self.shared {
					s.cancel.store(true, Ordering::SeqCst);
				}
			}
			let left = r.min_rect();
			let bar_w = (left.min.x - 12.0 - x0).max(50.0);
			let pr = prog.unwrap_or(if self.last_done.is_some() && !busy { 1.0 } else { 0.0 });
			let txt = Rect::from_min_size(Pos2::new(x0, y + 2.0), Vec2::new(bar_w, 16.0));
			paint_text_clipped(ui, Rect::from_min_max(txt.min, Pos2::new(txt.max.x - 60.0, txt.max.y)), &self.status, f_body(12.0), p.muted, Align2::LEFT_CENTER);
			let pct = if busy { format!("{}%", (pr * 100.0).round() as i32) } else if self.last_done.is_some() { tr("готово", "done").into() } else { String::new() };
			ui.painter().text(Pos2::new(x0 + bar_w, txt.center().y), Align2::RIGHT_CENTER, pct, f_body(12.0), p.muted);
			let bar = Rect::from_min_size(Pos2::new(x0, y + 22.0), Vec2::new(bar_w, 6.0));
			ui.painter().rect_filled(bar, 3, p.div2);
			if pr > 0.0 {
				ui.painter().rect_filled(Rect::from_min_size(bar.min, Vec2::new(bar_w * pr.clamp(0.0, 1.0), 6.0)), 3, p.accent);
			}
		}
		y += 32.0 + 16.0;

		let bottom_h = 40.0;
		let panel = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, full.max.y - bottom_h));
		ui.painter().rect(panel, 12, Color32::TRANSPARENT, Stroke::new(1.0, p.div), StrokeKind::Inside);
		let head = Rect::from_min_size(panel.min, Vec2::new(panel.width(), 40.0));
		{
			let mut tx = head.min.x + 8.0;
			for (i, t) in [tr("Справка", "Help"), tr("Журнал", "Log")].iter().enumerate() {
				let tw = ui.fonts(|f| f.layout_no_wrap(t.to_string(), f_body(13.5), p.text).size().x) + 24.0;
				let tr = Rect::from_min_size(Pos2::new(tx, head.center().y - 14.0), Vec2::new(tw, 28.0));
				let resp = ui.interact(tr, ui.id().with(("tab", i)), Sense::click());
				if self.tab == i {
					ui.painter().rect_filled(tr, 7, p.sel);
				} else if resp.hovered() {
					ui.painter().rect_filled(tr, 7, p.div2);
				}
				ui.painter().text(tr.center(), Align2::CENTER_CENTER, *t, f_body(13.5), if self.tab == i { p.text } else { p.muted });
				if resp.clicked() {
					self.tab = i;
				}
				tx += tw + 4.0;
			}
			ui.painter().text(Pos2::new(head.max.x - 14.0, head.center().y), Align2::RIGHT_CENTER, tr("Настройки и журнал: CrossbellTool.ini, CrossbellTool.log", "Settings and log: CrossbellTool.ini, CrossbellTool.log"), f_body(12.0), p.muted);
			ui.painter().line_segment([Pos2::new(panel.min.x + 1.0, head.max.y), Pos2::new(panel.max.x - 1.0, head.max.y)], Stroke::new(1.0, p.div));
		}
		let body = Rect::from_min_max(Pos2::new(panel.min.x + 1.0, head.max.y + 1.0), Pos2::new(panel.max.x - 1.0, panel.max.y - 1.0));
		if self.tab == 1 {
			ui.painter().rect_filled(body, egui::CornerRadius { nw: 0, ne: 0, sw: 11, se: 11 }, p.log);
		}
		let mut r = ui.new_child(egui::UiBuilder::new().max_rect(body.shrink2(Vec2::new(0.0, 0.0))));
		r.set_clip_rect(body);
		if self.tab == 0 {
			self.help(&mut r, &p);
		} else {
			self.log_view(&mut r, &p);
		}

		let br = Rect::from_min_max(Pos2::new(x0, full.max.y - bottom_h), Pos2::new(x1, full.max.y));
		let left = match sel_game(self.game_sel).or(det) {
			Some(g) => lformat!("Папки проекта и сборки — для {}", "Project and output folders: for {}", g.name()),
			None => tr("Перед сборкой сохраните оригиналы игры копией", "Back up the game's originals before building").to_string(),
		};
		let lg = ui.painter().layout_no_wrap(left, f_body(12.0), p.muted);
		let lw = lg.size().x;
		ui.painter().galley(Pos2::new(br.min.x, br.center().y - lg.size().y / 2.0), lg, p.muted);
		if let Some((m, t)) = &self.flash {
			if t.elapsed().as_secs_f32() < 1.8 {
				ui.painter().text(Pos2::new(br.min.x + lw + 16.0, br.center().y), Align2::LEFT_CENTER, m, f_body(12.0), p.ok_strong);
				ctx.request_repaint_after(std::time::Duration::from_millis(200));
			} else {
				self.flash = None;
			}
		}
		let mut r = ui.new_child(egui::UiBuilder::new().max_rect(br).layout(egui::Layout::right_to_left(egui::Align::Center)));
		if small_button(&mut r, &p, if ui_en() { "RU" } else { "EN" }, Some("languages"), 26.0, true, p.text).on_hover_text(tr("Switch to English", "Переключить на русский")).clicked() {
			let idle = self.status == tr("Готов к работе", "Ready");
			UI_EN.store(!ui_en(), std::sync::atomic::Ordering::Relaxed);
			self.save_settings();
			if idle {
				self.status = tr("Готов к работе", "Ready").into();
			}
		}
		if theme_button(&mut r, &p, 26.0).clicked() {
			self.pal = Pal::get(!self.pal.dark);
			apply_visuals(ctx, &self.pal);
			self.save_settings();
		}
	}

	fn help(&self, ui: &mut egui::Ui, p: &Pal) {
		let r = ui.max_rect();
		let inner = r.shrink2(Vec2::new(20.0, 16.0));
		let mut c = ui.new_child(egui::UiBuilder::new().max_rect(inner));
		egui::ScrollArea::vertical().id_salt("help").auto_shrink([false, false]).show(&mut c, |ui| {
			ui.label(egui::RichText::new(tr("CrossbellTool — декомпиляция и сборка The Legend of Heroes: Trails from Zero и Trails to Azure (PC). Игра определяется по папке (zero.exe, ao.exe) или выберите её в списке «Игра» — у каждой игры свои папки проекта. Сверка перевода — отдельная программа CrossbellCompare.exe.", "CrossbellTool decompiles and builds The Legend of Heroes: Trails from Zero and Trails to Azure (PC). The game is detected from the folder (zero.exe, ao.exe), or choose it in the 'Game' list. Each game has its own project folders. Translation comparison is a separate program, CrossbellCompare.exe.")).font(f_body(14.0)).color(p.muted));
			ui.add_space(14.0);
			let cards = [
				(tr("1 · Папка игры или файлы", "1 · Game folder or files"), tr("«Обзор…» → папка игры или русификатора (или перетащите её в окно). Программа просканирует её и откроет окно «Что разбирать»: отметьте галочками нужное. Открыть окно снова — кнопка «Файлы» справа. «Маска файлов» — только нужные файлы: c0100, t_item, c01* (пусто — все).", "'Browse…' → the game folder or a translation mod's folder (or drop it on the window). The program scans it and opens the 'What to decompile' window: tick what you need. To reopen that window, use the 'Files' button on the right. 'File mask' limits the work to specific files: c0100, t_item, c01* (empty means all).")),
				(tr("2 · Декомпилировать", "2 · Decompile"), tr("Разбирается то, что отмечено, — в «Куда сохранять для правки»: диалоги в .json, таблицы в .xlsx, текстуры в .png или .dds (формат — на втором шаге окна «Что разбирать»). Уже изменённые вами файлы не перезаписываются.", "The ticked items are unpacked into the 'Project folder': dialogues to .json, tables to .xlsx, textures to .png or .dds (the format is chosen in the second step of 'What to decompile'). Files you have already edited are not overwritten.")),
				(tr("3 · Собрать", "3 · Build"), tr("Откроется окно «Что собрать» — отметьте папки, видно, где сколько изменено. Собираются только изменённые файлы — в «Куда сохранять собранное». Скопируйте собранное в папку игры с заменой (оригиналы сохраните).", "The 'What to build' window opens: tick the folders and see how many files changed in each. Only changed files are built, into the 'Output folder'. Copy the result into the game folder, replacing the files (keep the originals).")),
			];
			let cw = (ui.available_width() - 2.0 * 12.0) / 3.0;
			let h = cards
				.iter()
				.map(|(_, t)| {
					let mut job = egui::text::LayoutJob::single_section(t.to_string(), egui::TextFormat { font_id: f_body(13.5), color: p.text, line_height: Some(20.0), ..Default::default() });
					job.wrap.max_width = cw - 28.0;
					ui.fonts(|f| f.layout_job(job)).size().y
				})
				.fold(0.0f32, f32::max)
				+ 24.0 + 30.0;
			let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
			for (i, (t, txt)) in cards.iter().enumerate() {
				let cr = Rect::from_min_size(Pos2::new(row.min.x + i as f32 * (cw + 12.0), row.min.y), Vec2::new(cw, h));
				ui.painter().rect(cr, 10, Color32::TRANSPARENT, Stroke::new(1.0, p.div), StrokeKind::Inside);
				ui.painter().text(Pos2::new(cr.min.x + 14.0, cr.min.y + 20.0), Align2::LEFT_CENTER, *t, f_semi(15.0), p.accent);
				let mut job = egui::text::LayoutJob::single_section(txt.to_string(), egui::TextFormat { font_id: f_body(13.5), color: p.text, line_height: Some(20.0), ..Default::default() });
				job.wrap.max_width = cw - 28.0;
				let g = ui.fonts(|f| f.layout_job(job));
				ui.painter().galley(Pos2::new(cr.min.x + 14.0, cr.min.y + 38.0), g, p.text);
			}
		});
	}

	fn log_view(&mut self, ui: &mut egui::Ui, p: &Pal) {
		let r = ui.max_rect();
		let inner = r.shrink2(Vec2::new(16.0, 10.0));
		let mut c = ui.new_child(egui::UiBuilder::new().max_rect(inner));
		let row_h = 12.5 * 1.65;
		let n = self.log.len();
		let mut sa = egui::ScrollArea::vertical().id_salt("log").auto_shrink([false, false]).stick_to_bottom(true);
		if self.log_to_end {
			sa = sa.vertical_scroll_offset((n as f32 * row_h - inner.height()).max(0.0));
			self.log_to_end = false;
		}
		c.spacing_mut().item_spacing.y = 0.0;
		sa.show_rows(&mut c, row_h, n, |ui, range| {
			ui.spacing_mut().item_spacing.y = 0.0;
			for i in range {
				let l = &self.log[i];
				let (rr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::hover());
				ui.painter().text(Pos2::new(rr.min.x, rr.center().y), Align2::LEFT_CENTER, &l.time, f_mono(12.5), p.muted);
				let col = match l.kind {
					Kind::Start => p.accent,
					Kind::Done => p.ok_strong,
					Kind::Warn => p.warn,
					Kind::Normal => p.text,
				};
				paint_text_clipped(ui, Rect::from_min_max(Pos2::new(rr.min.x + 72.0, rr.min.y), rr.max), &l.text, f_mono(12.5), col, Align2::LEFT_CENTER);
			}
		});
		if n == 0 {
			ui.painter().text(r.center(), Align2::CENTER_CENTER, tr("Журнал пуст — запустите декомпиляцию или сборку", "The log is empty: run a decompile or a build"), f_body(13.0), p.muted);
		}
	}
}

impl eframe::App for App {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		self.poll();
		if self.first_frame {
			self.first_frame = false;
		}
		let p = self.pal;
		let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
		let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
		if !dropped.is_empty() && self.shared.is_none() {
			self.src = dropped.iter().map(|x| x.display().to_string()).collect::<Vec<_>>().join("; ");
			self.save_settings();
			self.dec_sel = None;
			self.open_pick(false, ctx);
		}
		egui::CentralPanel::default().frame(egui::Frame::new().fill(p.bg)).show(ctx, |ui| {
			self.ui(ui, ctx);
		});
		if hovering {
			egui::Area::new(egui::Id::new("drop")).order(egui::Order::Foreground).fixed_pos(Pos2::ZERO).show(ctx, |ui| {
				let r = ctx.screen_rect().shrink(8.0);
				ui.painter().rect_filled(r, 12, p.bg.gamma_multiply(0.86));
				dashed_rect(ui, r, p.accent);
				ui.painter().text(r.center() - Vec2::new(0.0, 14.0), Align2::CENTER_CENTER, tr("Отпустите, чтобы указать папку игры", "Drop to set the game folder"), f_semi(26.0), p.accent);
				ui.painter().text(r.center() + Vec2::new(0.0, 18.0), Align2::CENTER_CENTER, tr("Папка игры или папка русификатора", "Game folder or a translation mod's folder"), f_body(14.0), p.muted);
			});
		}
		if let Some(pk) = &mut self.pick {
			let build = pk.mode == PMode::Build;
			if build && pk.empty() {
				self.pick = None;
				self.build_est = 0;
				self.build_ready = true;
				self.start(Op::Compile, ctx, false);
			} else {
				match pk.show(ctx, &p) {
					PEvent::None => {}
					PEvent::Cancel => self.pick = None,
					PEvent::Done => {
						let pk = self.pick.take().unwrap();
						if build {
							self.build_once = Picker::selected_keys(&pk.items);
							self.build_est = pk.out_estimate();
							self.build_ready = true;
							self.start(Op::Compile, ctx, false);
						} else {
							let n = pk.n_on();
							self.tex_dds = pk.tex_dds;
							self.save_settings();
							self.dec_sel = Some((pk.dir.clone(), pk.items));
							self.flash(&lformat!("Выбрано для разборки: {n}", "Selected to decompile: {n}"));
							if pk.then_start {
								self.start(Op::Decompile, ctx, false);
							}
						}
					}
				}
			}
		}
		match show_dialog(ctx, &p, "CrossbellTool", &mut self.dialog) {
			Some(Action::Start(op)) => self.start(op, ctx, true),
			Some(Action::ForceBuild) => {
				self.skip_check = true;
				self.build_once = self.last_build.clone();
				self.build_ready = true;
				self.start(Op::Compile, ctx, true);
			}
			None => {}
		}
		if self.shared.is_some() {
			ctx.request_repaint_after(std::time::Duration::from_millis(100));
		}
	}

	fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
		self.save_settings();
		if let Some(s) = &self.shared {
			s.cancel.store(true, Ordering::SeqCst);
			let t = Instant::now();
			while !s.finished.load(Ordering::SeqCst) && t.elapsed().as_secs() < 30 {
				std::thread::sleep(std::time::Duration::from_millis(50));
			}
		}
	}
}

fn same_dir(a: &std::path::Path, b: &std::path::Path) -> bool {
	if a == b {
		return true;
	}
	match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
		(Ok(x), Ok(y)) => x == y,
		_ => {
			let norm = |p: &std::path::Path| std::path::absolute(p).unwrap_or(p.to_path_buf()).to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
			norm(a) == norm(b)
		}
	}
}

fn dashed_rect(ui: &egui::Ui, r: Rect, c: Color32) {
	let s = Stroke::new(2.0, c);
	let p = ui.painter();
	let k = 12.0;
	let sides = [
		(Pos2::new(r.min.x + k, r.min.y), Pos2::new(r.max.x - k, r.min.y)),
		(Pos2::new(r.min.x + k, r.max.y), Pos2::new(r.max.x - k, r.max.y)),
		(Pos2::new(r.min.x, r.min.y + k), Pos2::new(r.min.x, r.max.y - k)),
		(Pos2::new(r.max.x, r.min.y + k), Pos2::new(r.max.x, r.max.y - k)),
	];
	for (a, b) in sides {
		p.extend(egui::Shape::dashed_line(&[a, b], s, 8.0, 6.0));
	}
	for (cx, cy, a0) in [(r.min.x + k, r.min.y + k, 180.0f32), (r.max.x - k, r.min.y + k, 270.0), (r.max.x - k, r.max.y - k, 0.0), (r.min.x + k, r.max.y - k, 90.0)] {
		let pts: Vec<Pos2> = (0..=8).map(|i| {
			let a = (a0 + i as f32 * 90.0 / 8.0).to_radians();
			Pos2::new(cx + k * a.cos(), cy + k * a.sin())
		}).collect();
		p.add(egui::Shape::line(pts, s));
	}
}

pub fn run() {
	let opts = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_title(TITLE)
			.with_inner_size([1000.0, 740.0])
			.with_min_inner_size([900.0, 640.0])
			.with_drag_and_drop(true)
			.with_icon(icon_from(include_bytes!("icons/crossbell_tool.png"))),
		centered: true,
		..Default::default()
	};
	if let Err(e) = eframe::run_native(TITLE, opts, Box::new(|cc| Ok(Box::new(App::new(cc))))) {
		eprintln!("{e}");
	}
}
