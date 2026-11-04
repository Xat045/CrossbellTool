#![allow(dead_code)]
#[allow(unused_imports)]
use crossbell_tool::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crossbell_tool::lang::tr;

use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::gui_common::*;
use crossbell_tool::kai::jobs::{self as kjobs, Counts};

pub fn groups() -> [(&'static str, &'static str); 8] {
	[
	(tr("Текст для перевода", "Text for translation"), tr("диалоги, таблицы, интерфейс", "dialogues, tables, interface")),
	(tr("Моды", "Mods"), tr("папка mods", "mods folder")),
	(tr("Текстуры", "Textures"), tr("картинки .png или .dds", ".png or .dds images")),
	(tr("Озвучка", "Voices"), tr("голоса персонажей", "character voices")),
	(tr("Музыка и звуки", "Music and sounds"), tr("звуковые и визуальные эффекты", "sound and visual effects")),
	(tr("Модели", "Models"), tr("персонажи, предметы, окружение", "characters, items, environment")),
	(tr("Видео", "Video"), tr("ролики", "movies")),
	(tr("Остальное", "Other"), tr("текста для перевода нет", "no text to translate")),
]
}
const G_TEXT: usize = 0;
const G_MODS: usize = 1;
const G_TEX: usize = 2;
const G_VOICE: usize = 3;
const G_SOUND: usize = 4;
const G_MODELS: usize = 5;
const G_VIDEO: usize = 6;
const G_OTHER: usize = 7;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
	Decompile,
	Build,
}

#[derive(Clone)]
pub struct Item {
	pub key: String,
	pub name: String,
	pub lang: String,
	pub desc: String,
	pub right: String,
	pub right_ok: bool,
	pub group: usize,
	pub on: bool,
	pub size: u64,
	pub unpacked: u64,
	pub changed: usize,
	pub out_key: String,
	pub out_size: u64,
	pub mod_size: u64,
}

pub enum Event {
	None,
	Cancel,
	Done,
}

static SPEED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn speed() -> f64 {
	f64::from_bits(SPEED.load(std::sync::atomic::Ordering::Relaxed))
}

pub fn set_speed(v: f64) {
	SPEED.store(v.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

static BUILD_SPEED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn build_speed() -> f64 {
	f64::from_bits(BUILD_SPEED.load(std::sync::atomic::Ordering::Relaxed))
}

pub fn set_build_speed(v: f64) {
	BUILD_SPEED.store(v.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

pub fn fmt_time(secs: f64) -> String {
	let m = (secs / 60.0).round() as u64;
	if m < 1 {
		tr("меньше минуты", "under a minute").into()
	} else if m < 60 {
		lformat!("≈ {m} мин", "≈ {m} min")
	} else {
		lformat!("≈ {} ч {} мин", "≈ {} h {} min", m / 60, m % 60)
	}
}

pub struct Picker {
	pub mode: Mode,
	pub dir: String,
	pub items: Vec<Item>,
	pub then_start: bool,
	pub mod_mode: bool,
	pub ed7: bool,
	pending: Option<Arc<Mutex<Option<Vec<Item>>>>>,
	pub tex_dds: bool,
	page: u8,
}

pub fn plural(n: usize, one: &str, few: &str, many: &str) -> String {
	let w = if crossbell_tool::lang::ui_en() {
		if n == 1 { one } else { many }
	} else if n % 10 == 1 && n % 100 != 11 {
		one
	} else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
		few
	} else {
		many
	};
	format!("{n} {w}")
}

pub fn fmt_size(b: u64) -> String {
	let mb = b as f64 / 1048576.0;
	if mb >= 1000.0 {
		lformat!("{:.1} ГБ", "{:.1} GB", mb / 1024.0).replace('.', ",")
	} else if mb >= 1.0 {
		lformat!("{} МБ", "{} MB", mb.round() as u64)
	} else {
		lformat!("{} КБ", "{} KB", ((b as f64) / 1024.0).round().max(1.0) as u64)
	}
}

fn describe(c: &Counts) -> String {
	let mut v = Vec::new();
	if c.scenes > 0 {
		v.push(plural(c.scenes, tr("сцена", "scene"), tr("сцены", "scenes"), tr("сцен", "scenes")));
	}
	if c.scripts > 0 {
		v.push(if c.scenes > 0 { plural(c.scripts, tr("другой скрипт", "other script"), tr("других скрипта", "other scripts"), tr("других скриптов", "other scripts")) } else { plural(c.scripts, tr("скрипт", "script"), tr("скрипта", "scripts"), tr("скриптов", "scripts")) });
	}
	if c.tables > 0 {
		v.push(plural(c.tables, tr("таблица", "table"), tr("таблицы", "tables"), tr("таблиц", "tables")));
	}
	if c.images > 0 {
		v.push(plural(c.images, tr("картинка", "image"), tr("картинки", "images"), tr("картинок", "images")));
	}
	if c.fonts > 0 {
		v.push(plural(c.fonts, tr("шрифт", "font"), tr("шрифта", "fonts"), tr("шрифтов", "fonts")));
	}
	if c.yaml > 0 {
		v.push(plural(c.yaml, tr("файл интерфейса", "interface file"), tr("файла интерфейса", "interface files"), tr("файлов интерфейса", "interface files")));
	}
	if c.other > 0 {
		v.push(if v.is_empty() { plural(c.other, tr("файл", "file"), tr("файла", "files"), tr("файлов", "files")) } else { plural(c.other, tr("другой файл", "other file"), tr("других файла", "other files"), tr("других файлов", "other files")) });
	}
	if v.is_empty() {
		tr("пусто", "empty").into()
	} else {
		v.join(", ")
	}
}

fn eng_suffix(s: &str) -> bool {
	s.split('/').any(|seg| seg.ends_with("_eng") || seg.ends_with("_en") || seg.ends_with("_us"))
}

fn first_seg(s: &str) -> &str {
	s.split('/').next().unwrap_or(s)
}

fn sound_name(s: &str) -> bool {
	s.split('/').any(|seg| seg == "se" || seg.starts_with("se_") || seg.starts_with("vfx") || seg.starts_with("bgm") || seg.starts_with("sound") || seg.starts_with("music"))
}

fn game_item(s: kjobs::ScanItem) -> Item {
	let stem = s.key.trim_end_matches(".p3a").to_string();
	let base = stem.rsplit('/').next().unwrap_or(&stem).to_string();
	let c = &s.counts;
	let group = if s.in_mods {
		G_MODS
	} else if base == "multilanguage" || base.starts_with("script") || base.starts_with("table") || c.scenes + c.tables > 0 {
		G_TEXT
	} else if base.contains("image") {
		G_TEX
	} else if base.starts_with("voice") {
		G_VOICE
	} else if sound_name(&base) {
		G_SOUND
	} else if base.contains("model") || base.contains("cubemap") {
		G_MODELS
	} else if base.starts_with("movie") {
		G_VIDEO
	} else {
		G_OTHER
	};
	let lang = if s.in_mods {
		String::new()
	} else if base == "multilanguage" {
		let en = s.files.iter().any(|f| f.to_lowercase().contains("_en"));
		let jp = s.files.iter().any(|f| f.to_lowercase().contains("_jp") || f.to_lowercase().contains("_ja"));
		match (en, jp) {
			(true, true) => tr("Английские и японские файлы", "English and Japanese files").into(),
			(true, false) => tr("Английские файлы", "English files").into(),
			(false, true) => tr("Японские файлы", "Japanese files").into(),
			_ => String::new(),
		}
	} else if eng_suffix(&base) {
		tr("Английские файлы", "English files").into()
	} else if group == G_TEXT || group == G_VOICE {
		tr("Японские файлы", "Japanese files").into()
	} else if group == G_TEX {
		tr("Японские и общие файлы", "Japanese and common files").into()
	} else {
		String::new()
	};
	let n = c.total();
	let desc = if let Some(e) = &s.error {
		lformat!("Не удалось прочитать: {e}", "Could not read: {e}")
	} else if base == "multilanguage" {
		lformat!("Найдено: {} — надписи интерфейса", "Found: {}, interface text", s.files.join(", "))
	} else {
		match group {
			G_TEXT | G_MODS => lformat!("Найдено: {}", "Found: {}", describe(c)),
			G_TEX => lformat!("{} → .png или .dds", "{} → .png or .dds", plural(c.images.max(n), tr("картинка", "image"), tr("картинки", "images"), tr("картинок", "images"))),
			G_VOICE => lformat!("Озвучка · {}", "Voices · {}", plural(n, tr("файл", "file"), tr("файла", "files"), tr("файлов", "files"))),
			G_SOUND if base.starts_with("vfx") => lformat!("Визуальные эффекты · {}", "Visual effects · {}", plural(n, tr("файл", "file"), tr("файла", "files"), tr("файлов", "files"))),
			G_SOUND => lformat!("Звуковые эффекты · {}", "Sound effects · {}", plural(n, tr("файл", "file"), tr("файла", "files"), tr("файлов", "files"))),
			G_MODELS if base.contains("info") => lformat!("Описания моделей · {}", "Model descriptions · {}", plural(n, tr("файл", "file"), tr("файла", "files"), tr("файлов", "files"))),
			G_MODELS if base.contains("cubemap") => lformat!("Карты окружения · {}", "Environment maps · {}", plural(n, tr("файл", "file"), tr("файла", "files"), tr("файлов", "files"))),
			G_MODELS => lformat!("Модели · {}", "Models · {}", plural(n, tr("файл", "file"), tr("файла", "files"), tr("файлов", "files"))),
			_ => lformat!("{} — текста для перевода нет", "{}: no text to translate", plural(n, tr("файл", "file"), tr("файла", "files"), tr("файлов", "files"))),
		}
	};
	Item { key: s.key, name: s.name, lang, desc, right: fmt_size(s.size), right_ok: false, group, on: true, size: s.size, unpacked: s.unpacked, changed: 0, out_key: String::new(), out_size: 0, mod_size: 0 }
}

fn proj_item(s: kjobs::ProjItem) -> Item {
	let low = s.name.to_lowercase().replace('\\', "/");
	let kind = s.kind.as_str();
	let group = if kind.starts_with("script") || kind.starts_with("table") || kind == "multilanguage" {
		G_TEXT
	} else if low.contains("voice") {
		G_VOICE
	} else if low.contains("model") || low.contains("cubemap") {
		G_MODELS
	} else if sound_name(&low) {
		G_SOUND
	} else if low.contains("movie") {
		G_VIDEO
	} else if s.counts.images > 0 || kind.starts_with("asset") {
		G_TEX
	} else {
		G_OTHER
	};
	let lang = if s.in_mods {
		tr("Русский перевод (мод)", "Russian translation (mod)").to_string()
	} else if kind == "multilanguage" {
		String::new()
	} else if eng_suffix(&low) {
		tr("Английские файлы", "English files").to_string()
	} else if group == G_TEXT || group == G_VOICE {
		tr("Японские файлы", "Japanese files").to_string()
	} else {
		String::new()
	};
	let mut desc = describe(&s.counts);
	if s.counts.scenes + s.counts.scripts > 0 && s.counts.tables == 0 && s.counts.images == 0 {
		desc.push_str(" (.json)");
	} else if s.counts.tables > 0 && s.counts.scenes + s.counts.scripts + s.counts.images == 0 {
		desc.push_str(" (.xlsx)");
	}
	let _ = first_seg;
	let (right, ok) = if s.changed > 0 { (lformat!("изменено {}", "{} changed", s.changed), true) } else { (tr("без изменений", "no changes").to_string(), false) };
	Item { key: s.key, name: s.name, lang, desc, right, right_ok: ok, group, on: true, size: 0, unpacked: 0, changed: s.changed, out_key: s.archive, out_size: s.archive_size, mod_size: s.mod_size }
}

fn ed7_items(v: Vec<crossbell_tool::jobs::Ed7Folder>, mode: Mode) -> Vec<Item> {
	let keys: std::collections::HashSet<String> = v.iter().map(|f| f.key.clone()).collect();
	let decomp = mode == Mode::Decompile;
	let mut out: Vec<Item> = v
		.into_iter()
		.filter(|f| f.scenes + f.tables + f.textures > 0)
		.map(|f| {
			let text = f.scenes + f.tables > 0;
			let group = if text { G_TEXT } else { G_TEX };
			let us = f.key.ends_with("_us");
			let lang = if us {
				tr("Английские файлы", "English files")
			} else if text {
				tr("Японские файлы", "Japanese files")
			} else if keys.contains(&format!("{}_us", f.key)) {
				tr("Японские и общие файлы", "Japanese and common files")
			} else {
				tr("Общие файлы", "Common files")
			};
			let mut parts = Vec::new();
			if f.scenes > 0 {
				parts.push(format!("{} {}", plural(f.scenes, tr("сцена", "scene"), tr("сцены", "scenes"), tr("сцен", "scenes")), if decomp { ".bin → .json" } else { ".json → .bin" }));
			}
			if f.tables > 0 {
				parts.push(format!("{} {}", plural(f.tables, tr("таблица", "table"), tr("таблицы", "tables"), tr("таблиц", "tables")), if decomp { "._dt → .xlsx" } else { ".xlsx → ._dt" }));
			}
			if f.textures > 0 {
				parts.push(format!("{} {}", plural(f.textures, tr("картинка", "image"), tr("картинки", "images"), tr("картинок", "images")), if decomp { tr(".itp → .png или .dds", ".itp → .png or .dds") } else { ".png / .dds → .itp" }));
			}
			let desc = if decomp { lformat!("Найдено: {}", "Found: {}", parts.join(", ")) } else { parts.join(", ") };
			let (right, ok) = if decomp {
				(fmt_size(f.size), false)
			} else if f.changed > 0 {
				(lformat!("изменено {}", "{} changed", f.changed), true)
			} else {
				(tr("без изменений", "no changes").to_string(), false)
			};
			Item { key: f.key.clone(), name: format!("{}\\", f.name), lang: lang.to_string(), desc, right, right_ok: ok, group, on: true, size: f.size, unpacked: f.unpacked, changed: f.changed, out_key: f.key.clone(), out_size: f.out_size, mod_size: 0 }
		})
		.collect();
	out.sort_by_key(|i| (i.group, i.key.trim_end_matches("_us").to_string(), !i.key.ends_with("_us")));
	out
}

impl Picker {
	pub fn open(mode: Mode, dir: &str, then_start: bool, mod_mode: bool, ctx: &egui::Context) -> Picker {
		let mut p = Picker { mode, dir: dir.to_string(), items: Vec::new(), then_start, mod_mode, ed7: false, pending: None, tex_dds: false, page: 0 };
		p.rescan(ctx);
		p
	}

	pub fn with_items(mode: Mode, dir: &str, items: Vec<Item>, then_start: bool, mod_mode: bool) -> Picker {
		Picker { mode, dir: dir.to_string(), items, then_start, mod_mode, ed7: false, pending: None, tex_dds: false, page: 0 }
	}

	pub fn open_ed7(mode: Mode, dir: &str, then_start: bool, ctx: &egui::Context) -> Picker {
		let mut p = Picker { mode, dir: dir.to_string(), items: Vec::new(), then_start, mod_mode: false, ed7: true, pending: None, tex_dds: false, page: 0 };
		p.rescan(ctx);
		p
	}

	pub fn with_items_ed7(mode: Mode, dir: &str, items: Vec<Item>, then_start: bool) -> Picker {
		Picker { mode, dir: dir.to_string(), items, then_start, mod_mode: false, ed7: true, pending: None, tex_dds: false, page: 0 }
	}

	pub fn rescan(&mut self, ctx: &egui::Context) {
		let slot = Arc::new(Mutex::new(None));
		self.pending = Some(slot.clone());
		self.items.clear();
		let dir = PathBuf::from(&self.dir);
		let mode = self.mode;
		let ed7 = self.ed7;
		let ctx = ctx.clone();
		std::thread::spawn(move || {
			if ed7 {
				let v = match mode {
					Mode::Decompile => crossbell_tool::jobs::scan_game_ed7(&dir),
					Mode::Build => crossbell_tool::jobs::scan_project_ed7(&dir),
				};
				*slot.lock().unwrap() = Some(ed7_items(v, mode));
				ctx.request_repaint();
				return;
			}
			let items: Vec<Item> = match mode {
				Mode::Decompile => kjobs::scan_game(&dir).into_iter().map(game_item).collect(),
				Mode::Build => kjobs::scan_project(&dir).into_iter().map(proj_item).collect(),
			};
			let mut items = items;
			items.sort_by_key(|i| (i.group, i.name.ends_with('\\')));
			*slot.lock().unwrap() = Some(items);
			ctx.request_repaint();
		});
	}

	pub fn scanning(&self) -> bool {
		self.pending.is_some()
	}

	fn poll(&mut self) {
		let got = self.pending.as_ref().and_then(|slot| slot.lock().unwrap().take());
		if let Some(v) = got {
			self.items = v;
			self.pending = None;
		}
	}

	pub fn empty(&mut self) -> bool {
		self.poll();
		!self.scanning() && self.items.is_empty()
	}

	pub fn out_estimate(&self) -> u64 {
		let on = self.items.iter().filter(|i| i.on);
		if self.ed7 {
			on.map(|i| i.out_size).sum()
		} else if self.mod_mode {
			self.items.first().map(|i| i.mod_size).unwrap_or(0)
		} else {
			let mut seen = std::collections::HashSet::new();
			on.filter(|i| i.changed > 0 && !i.out_key.is_empty() && seen.insert(i.out_key.clone())).map(|i| i.out_size).sum()
		}
	}

	pub fn n_on(&self) -> usize {
		self.items.iter().filter(|i| i.on).count()
	}

	pub fn selected_keys(items: &[Item]) -> Option<Vec<String>> {
		if items.iter().all(|i| i.on) {
			None
		} else {
			Some(items.iter().filter(|i| i.on).map(|i| i.key.clone()).collect())
		}
	}

	fn preset(&mut self, groups: Option<&[usize]>) {
		for i in &mut self.items {
			i.on = match groups {
				Some(g) => g.contains(&i.group),
				None => true,
			};
		}
	}

	fn preset_active(&self, groups: Option<&[usize]>) -> bool {
		!self.items.is_empty()
			&& self.items.iter().all(|i| i.on == match groups {
				Some(g) => g.contains(&i.group),
				None => true,
			})
	}

	pub fn show(&mut self, ctx: &egui::Context, p: &Pal) -> Event {
		self.poll();
		let mut ev = Event::None;
		let screen = ctx.screen_rect();
		egui::Area::new(egui::Id::new("pick_dim")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
			let (r, _) = ui.allocate_exact_size(screen.size(), Sense::click());
			ui.painter().rect_filled(r, 0, Color32::from_black_alpha(if p.dark { 120 } else { 70 }));
		});
		let w = 820f32.min(screen.width() - 40.0);
		let h = 720f32.min(screen.height() - 40.0);
		let rect = Rect::from_center_size(screen.center(), Vec2::new(w, h));
		let decomp = self.mode == Mode::Decompile;
		egui::Area::new(egui::Id::new("pick_box")).order(egui::Order::Foreground).fixed_pos(rect.min).show(ctx, |ui| {
			let (_, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
			let painter = ui.painter().clone();
			painter.add(egui::epaint::Shadow { offset: [0, 14], blur: 40, spread: 0, color: Color32::from_black_alpha(70) }.as_shape(rect, 14));
			painter.rect(rect, 14, p.bg, Stroke::new(1.0, p.div), StrokeKind::Inside);
			let x0 = rect.min.x + 24.0;
			let x1 = rect.max.x - 24.0;
			let mut y = rect.min.y + 20.0;

			let fmt_page = decomp && self.page == 1;
			painter.text(Pos2::new(x0, y + 11.0), Align2::LEFT_CENTER, if fmt_page { tr("Формат текстур", "Texture format") } else if decomp { tr("Что разбирать", "What to decompile") } else { tr("Что собрать", "What to build") }, f_semi(19.0), p.text);
			{
				let t = if self.scanning() || fmt_page { "" } else { tr("Сканировать заново", "Rescan") };
				let g = painter.layout_no_wrap(t.to_string(), f_body(13.0), p.accent);
				let tr = Rect::from_min_size(Pos2::new(x1 - g.size().x, y + 11.0 - g.size().y / 2.0), g.size());
				if !t.is_empty() {
					let resp = ui.interact(tr, ui.id().with("rescan"), Sense::click());
					painter.galley(tr.min, g, if resp.hovered() { p.accent_strong } else { p.accent });
					if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
						self.rescan(ctx);
					}
				}
			}
			y += 30.0;
			let n_all = self.items.len();
			let headline = if fmt_page {
				tr("Отмечены текстуры — в каком виде их разобрать для правки?", "Textures are ticked: in which format should they be unpacked for editing?").to_string()
			} else if self.scanning() {
				if decomp { tr("Сканирую папку…", "Scanning the folder…").to_string() } else { tr("Смотрю папку проекта…", "Reading the project folder…").to_string() }
			} else if decomp {
				if n_all == 0 {
					if self.ed7 {
						tr("Подходящих папок не найдено — укажите папку игры или русификатора (в её data лежат scena_us, text_us…)", "No suitable folders found: choose the game folder or a translation mod's folder (its data folder holds scena_us, text_us…)").to_string()
					} else {
						tr("Подходящих архивов не найдено — укажите корневую папку игры (где лежат .p3a и папка mods)", "No suitable archives found: choose the game's root folder (the one with the .p3a files and the mods folder)").to_string()
					}
				} else {
					let total: u64 = self.items.iter().map(|i| i.size).sum();
					lformat!("Найдено {} ({}). По умолчанию отмечено всё — снимите лишнее.", "Found {} ({}). Everything is ticked by default: untick what you don't need.", plural(n_all, tr("пункт", "item"), tr("пункта", "items"), tr("пунктов", "items")), fmt_size(total))
				}
			} else {
				let ch: usize = self.items.iter().map(|i| i.changed).sum();
				lformat!("{} · {}, изменено {}", "{} · {}, {} changed", self.dir, plural(n_all, tr("папка", "folder"), tr("папки", "folders"), tr("папок", "folders")), plural(ch, tr("файл", "file"), tr("файла", "files"), tr("файлов", "files")))
			};
			paint_text_clipped(ui, Rect::from_min_size(Pos2::new(x0, y), Vec2::new(x1 - x0, 18.0)), &headline, f_body(13.0), p.muted, Align2::LEFT_CENTER);
			y += 26.0;
			if self.scanning() {
				let bar = Rect::from_min_size(Pos2::new(x0, y + 4.0), Vec2::new(x1 - x0, 6.0));
				painter.rect_filled(bar, 3, p.div2);
				let t = ctx.input(|i| i.time) as f32;
				let k = (t * 0.8).fract();
				let bw = (x1 - x0) * 0.3;
				let bx = x0 - bw + (x1 - x0 + bw) * k;
				painter.with_clip_rect(bar).rect_filled(Rect::from_min_size(Pos2::new(bx, bar.min.y), Vec2::new(bw, 6.0)), 3, p.accent);
				ctx.request_repaint();
				y += 20.0;
			} else if fmt_page {
				y += 30.0 + 12.0;
			} else {
				let text: &[usize] = if decomp { &[G_TEXT, G_MODS] } else { &[G_TEXT] };
				let tex: &[usize] = if decomp { &[G_TEXT, G_MODS, G_TEX] } else { &[G_TEXT, G_TEX] };
				let chips: [(&str, Option<&[usize]>); 4] = [(tr("Только текст", "Text only"), Some(text)), (tr("Текст+текстуры", "Text+textures"), Some(tex)), (tr("Всё", "All"), None), (tr("Снять всё", "Clear all"), Some(&[]))];
				let mut cx = x0;
				for (i, (label, g)) in chips.iter().enumerate() {
					let tw = painter.layout_no_wrap(label.to_string(), f_body(13.0), p.text).size().x + 28.0;
					let cr = Rect::from_min_size(Pos2::new(cx, y), Vec2::new(tw, 30.0));
					let resp = ui.interact(cr, ui.id().with(("chip", i)), Sense::click());
					let active = i < 3 && self.preset_active(*g);
					let (bg, fg, st) = if active { (p.sel, p.accent_strong, p.accent) } else if resp.hovered() { (p.div2, p.text, p.div) } else { (Color32::TRANSPARENT, p.text, p.div) };
					painter.rect(cr, 15, bg, Stroke::new(1.0, st), StrokeKind::Inside);
					painter.text(cr.center(), Align2::CENTER_CENTER, *label, f_body(13.0), fg);
					if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
						self.preset(*g);
					}
					cx += tw + 8.0;
				}
				y += 30.0 + 12.0;
			}
			painter.line_segment([Pos2::new(rect.min.x + 1.0, y), Pos2::new(rect.max.x - 1.0, y)], Stroke::new(1.0, p.div));

			let foot_h = 66.0;
			let foot = Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - foot_h), rect.max);
			painter.line_segment([Pos2::new(rect.min.x + 1.0, foot.min.y), Pos2::new(rect.max.x - 1.0, foot.min.y)], Stroke::new(1.0, p.div));

			let list = Rect::from_min_max(Pos2::new(rect.min.x + 12.0, y + 1.0), Pos2::new(rect.max.x - 6.0, foot.min.y - 1.0));
			let mut lu = ui.new_child(egui::UiBuilder::new().max_rect(list));
			lu.set_clip_rect(list);
			if fmt_page {
				let ed7 = self.ed7;
				let opts: [(bool, &str, [&str; 2]); 2] = [
					(false, tr(".png — обычные картинки", ".png: ordinary images"), [tr("Открываются в любом редакторе: Photoshop, GIMP, Paint.NET, Krita.", "They open in any editor: Photoshop, GIMP, Paint.NET, Krita."), tr("При сборке программа сожмёт их обратно в формат игры.", "When building, the program compresses them back into the game's format.")]),
					(
						true,
						if ed7 { tr(".dds — данные игры без перекодирования", ".dds: game data without re-encoding") } else { tr(".dds — как в игре, без перевода в .png", ".dds: as in the game, no .png conversion") },
						[
							if ed7 { tr("Те же сжатые блоки BC7, что в .itp, со всеми уменьшенными копиями — без потерь.", "The same BC7 compressed blocks as in .itp, with all mipmaps: lossless.") } else { tr("Файлы игры копируются как есть — быстрее и без потерь качества.", "Game files are copied as they are: faster and with no quality loss.") },
							tr("Нужен редактор с поддержкой DDS (Paint.NET, GIMP, плагин для Photoshop).", "You need an editor with DDS support (Paint.NET, GIMP, a Photoshop plugin)."),
						],
					),
				];
				let mut cy = list.min.y + 16.0;
				for (i, (dds, title, lines)) in opts.iter().enumerate() {
					let cr = Rect::from_min_size(Pos2::new(list.min.x + 12.0, cy), Vec2::new(list.width() - 30.0, 92.0));
					let resp = ui.interact(cr, ui.id().with(("texfmt", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
					let on = self.tex_dds == *dds;
					painter.rect(cr, 10, if on { p.sel.gamma_multiply(if p.dark { 0.6 } else { 0.55 }) } else if resp.hovered() { p.div2 } else { Color32::TRANSPARENT }, Stroke::new(if on { 1.5 } else { 1.0 }, if on { p.accent } else { p.div }), StrokeKind::Inside);
					let c = Pos2::new(cr.min.x + 26.0, cr.min.y + 26.0);
					painter.circle_stroke(c, 9.0, Stroke::new(1.5, if on { p.accent } else { p.muted }));
					if on {
						painter.circle_filled(c, 5.0, p.accent);
					}
					painter.text(Pos2::new(cr.min.x + 48.0, c.y), Align2::LEFT_CENTER, *title, f_semi(15.0), p.text);
					for (j, l) in lines.iter().enumerate() {
						paint_text_clipped(ui, Rect::from_min_size(Pos2::new(cr.min.x + 48.0, cr.min.y + 44.0 + j as f32 * 20.0), Vec2::new(cr.width() - 60.0, 18.0)), l, f_body(12.5), p.muted, Align2::LEFT_CENTER);
					}
					if resp.clicked() {
						self.tex_dds = *dds;
					}
					cy += 92.0 + 12.0;
				}
			} else {
			egui::ScrollArea::vertical().id_salt("pick_list").auto_shrink([false, false]).show(&mut lu, |ui| {
				ui.spacing_mut().item_spacing.y = 0.0;
				let rw = ui.available_width() - 8.0;
				ui.add_space(6.0);
				for g in 0..groups().len() {
					let idx: Vec<usize> = (0..self.items.len()).filter(|&i| self.items[i].group == g).collect();
					if idx.is_empty() {
						continue;
					}
					let on = idx.iter().filter(|&&i| self.items[i].on).count();
					ui.add_space(8.0);
					let (hr, resp) = ui.allocate_exact_size(Vec2::new(rw, 30.0), Sense::click());
					if resp.hovered() {
						ui.painter().rect_filled(hr, 7, p.div2);
					}
					let state = if on == 0 { 0 } else if on == idx.len() { 1 } else { 2 };
					tri_box(ui, Rect::from_center_size(Pos2::new(hr.min.x + 21.0, hr.center().y), Vec2::splat(18.0)), state, p);
					let title = groups()[g].0.to_uppercase();
					let tg = ui.fonts(|f| f.layout_job(caps_job(&title, p.muted)));
					let tw = tg.size().x;
					ui.painter().galley(Pos2::new(hr.min.x + 40.0, hr.center().y - tg.size().y / 2.0), tg, p.muted);
					let note = if g == G_OTHER && !decomp {
						tr("попадут в сборку, только если изменены", "only go into the build if changed")
					} else if self.ed7 && g == G_TEXT {
						tr("диалоги и таблицы", "dialogues and tables")
					} else if self.ed7 && g == G_TEX {
						if decomp { tr("картинки .itp → .png или .dds", ".itp images → .png or .dds") } else { tr("картинки .png / .dds → .itp", ".png / .dds images → .itp") }
					} else {
						groups()[g].1
					};
					ui.painter().text(Pos2::new(hr.min.x + 52.0 + tw, hr.center().y), Align2::LEFT_CENTER, note, f_body(12.0), p.muted.gamma_multiply(0.85));
					ui.painter().text(Pos2::new(hr.max.x - 12.0, hr.center().y), Align2::RIGHT_CENTER, lformat!("{on} из {}", "{on} of {}", idx.len()), f_body(12.0), p.muted);
					if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
						let v = on != idx.len();
						for &i in &idx {
							self.items[i].on = v;
						}
					}
					for &i in &idx {
						let it = &self.items[i];
						let has_lang = !it.lang.is_empty();
						let rh = if has_lang { 62.0 } else { 46.0 };
						let (rr, resp) = ui.allocate_exact_size(Vec2::new(rw, rh), Sense::click());
						let bg = if it.on { p.sel.gamma_multiply(if p.dark { 0.6 } else { 0.55 }) } else { Color32::TRANSPARENT };
						ui.painter().rect_filled(rr, 8, if resp.hovered() && !it.on { p.div2 } else { bg });
						tri_box(ui, Rect::from_center_size(Pos2::new(rr.min.x + 49.0, rr.center().y), Vec2::splat(18.0)), if it.on { 1 } else { 0 }, p);
						let tx = rr.min.x + 70.0;
						let right_w = 130.0;
						let tr = Rect::from_min_max(Pos2::new(tx, rr.min.y), Pos2::new(rr.max.x - right_w, rr.max.y));
						let mut ty = rr.min.y + 8.0;
						paint_text_clipped(ui, Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(tr.width(), 18.0)), &it.name, f_mono(13.0), p.text, Align2::LEFT_CENTER);
						ty += 18.0;
						if has_lang {
							paint_text_clipped(ui, Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(tr.width(), 16.0)), &it.lang, f_semi(12.0), p.accent, Align2::LEFT_CENTER);
							ty += 16.0;
						}
						paint_text_clipped(ui, Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(tr.width(), 16.0)), &it.desc, f_body(12.0), p.muted, Align2::LEFT_CENTER);
						let rc = if it.right_ok { p.ok_strong } else { p.muted };
						ui.painter().text(Pos2::new(rr.max.x - 12.0, rr.center().y), Align2::RIGHT_CENTER, &it.right, f_body(12.5), rc);
						if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
							self.items[i].on = !self.items[i].on;
						}
					}
				}
				ui.add_space(10.0);
			});
			}

			let n_on = self.n_on();
			let has_tex = decomp && self.items.iter().any(|i| i.on && i.group == G_TEX);
			let (summary, hint) = if fmt_page {
				(lformat!("Текстуры — в {}", "Textures: as {}", if self.tex_dds { ".dds" } else { ".png" }), tr("Выбор запоминается — в следующий раз будет отмечен он же", "The choice is remembered and will be ticked next time").to_string())
			} else if self.scanning() {
				(tr("Идёт сканирование…", "Scanning…").to_string(), String::new())
			} else if n_on == 0 {
				(tr("Ничего не выбрано", "Nothing selected").to_string(), if decomp { tr("Отметьте хотя бы один пункт", "Tick at least one item").to_string() } else { tr("Отметьте хотя бы одну папку", "Tick at least one folder").to_string() })
			} else if decomp {
				let size: u64 = self.items.iter().filter(|i| i.on).map(|i| i.size).sum();
				let h = if size > (1u64 << 30) { tr("Большой объём — распаковка займёт время и место на диске", "Large amount: unpacking will take time and disk space").to_string() } else { tr("Займёт немного времени", "Won't take long").to_string() };
				let unp: u64 = self.items.iter().filter(|i| i.on).map(|i| i.unpacked).sum();
				let after = if unp > 0 { lformat!(" → разобранные ≈ {}", " → unpacked ≈ {}", fmt_size(unp)) } else { String::new() };
				let h = match (unp > 0, speed()) {
					(true, v) if v > 0.0 => lformat!("{h} · займёт {}", "{h} · takes {}", fmt_time(unp as f64 / v)),
					(true, _) => lformat!("{h} · время — после первой разборки", "{h} · time is shown after the first decompile"),
					_ => h,
				};
				(lformat!("Выбрано {n_on} из {n_all} · {}{after}", "Selected {n_on} of {n_all} · {}{after}", fmt_size(size)), h)
			} else {
				let ch: usize = self.items.iter().filter(|i| i.on).map(|i| i.changed).sum();
				let h = if self.ed7 { tr("Неотмеченные папки не собираются", "Unticked folders are not built").to_string() } else if self.mod_mode { tr("Неотмеченные папки войдут в мод без ваших правок", "Unticked folders go into the mod without your edits").to_string() } else { tr("Неотмеченные папки не пересобираются", "Unticked folders are not rebuilt").to_string() };
				let out = self.out_estimate();
				let after = if out > 0 { lformat!(" → запишется ≈ {}", " → writes ≈ {}", fmt_size(out)) } else { String::new() };
				let h = match (out > 0, build_speed()) {
					(true, v) if v > 0.0 => lformat!("{h} · займёт {}", "{h} · takes {}", fmt_time(out as f64 / v)),
					(true, _) => lformat!("{h} · время — после первой сборки", "{h} · time is shown after the first build"),
					_ => h,
				};
				(lformat!("Собрать {n_on} из {n_all} · изменено {}{after}", "Build {n_on} of {n_all} · {} changed{after}", plural(ch, tr("файл", "file"), tr("файла", "files"), tr("файлов", "files"))), h)
			};
			painter.text(Pos2::new(x0, foot.center().y - 9.0), Align2::LEFT_CENTER, summary, f_semi(14.0), p.text);
			painter.text(Pos2::new(x0, foot.center().y + 11.0), Align2::LEFT_CENTER, hint, f_body(12.0), p.muted);
			let bh = 38.0;
			let next = has_tex && !fmt_page;
			let ok_label = if next { tr("Далее", "Next") } else if decomp { tr("Готово", "Done") } else { tr("Собрать", "Build") };
			let ok_w = 120.0;
			let ok_r = Rect::from_min_size(Pos2::new(x1 - ok_w, foot.center().y - bh / 2.0), Vec2::new(ok_w, bh));
			let can = !self.scanning() && n_on > 0;
			let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(ok_r));
			let ok_icon = if next { "arrow-right" } else if decomp { "check" } else { "archive" };
			if primary_button_icon(&mut bu, p, ok_label, Some(ok_icon), ok_r.size(), 8, f_semi(14.5), can).clicked() {
				if next {
					self.page = 1;
				} else {
					ev = Event::Done;
				}
			}
			let cancel_r = Rect::from_min_max(Pos2::new(x0 + 300.0, ok_r.min.y), Pos2::new(ok_r.min.x - 10.0, ok_r.max.y));
			let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(cancel_r).layout(egui::Layout::right_to_left(egui::Align::Center)));
			if small_button(&mut cu, p, if fmt_page { tr("Назад", "Back") } else { tr("Отмена", "Cancel") }, Some(if fmt_page { "arrow-left" } else { "x" }), bh, true, p.text).clicked() {
				if fmt_page {
					self.page = 0;
				} else {
					ev = Event::Cancel;
				}
			}
		});
		if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
			ev = Event::Cancel;
		}
		ev
	}
}

fn tri_box(ui: &egui::Ui, b: Rect, state: u8, p: &Pal) {
	if state == 0 {
		ui.painter().rect(b, 5, Color32::TRANSPARENT, Stroke::new(1.5, p.muted), StrokeKind::Inside);
	} else {
		ui.painter().rect_filled(b, 5, p.accent);
		if state == 1 {
			paint_icon(ui, "check", b.shrink(2.5), p.on_accent);
		} else {
			ui.painter().rect_filled(Rect::from_center_size(b.center(), Vec2::new(8.0, 2.0)), 1, p.on_accent);
		}
	}
}
