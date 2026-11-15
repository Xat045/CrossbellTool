#![allow(dead_code)]
#[allow(unused_imports)]
use crossbell_tool::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crossbell_tool::lang::tr;

use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::gui_common::*;

pub fn groups() -> [(&'static str, &'static str); 12] {
	[
	(tr("Текст для перевода", "Text for translation"), tr("диалоги и таблицы", "dialogues and tables")),
	(tr("Интерфейс и меню", "Interface and menus"), tr("меню, экраны, надписи, значки", "menus, screens, captions, icons")),
	(tr("Мини-игры и записки", "Mini-games and notes"), tr("мини-игры, рецепты, рыбалка", "mini-games, recipes, fishing")),
	(tr("Картинки", "Pictures"), tr("иллюстрации, книги, плакаты", "illustrations, books, posters")),
	(tr("Портреты", "Portraits"), tr("лица в диалогах", "faces in dialogue")),
	(tr("Бой и эффекты", "Battle and effects"), tr("бой, эффекты", "battle, effects")),
	(tr("Персонажи и монстры", "Characters and monsters"), tr("модели и анимации", "models and animations")),
	(tr("Карты", "Maps"), tr("локации и их текстуры", "locations and their textures")),
	(tr("Озвучка", "Voices"), tr("голоса персонажей", "character voices")),
	(tr("Музыка и звуки", "Music and sounds"), tr("музыка и звуковые эффекты", "music and sound effects")),
	(tr("Видео", "Video"), tr("ролики", "movies")),
	(tr("Остальное", "Other"), tr("текста для перевода нет", "no text to translate")),
]
}
const G_TEXT: usize = 0;
const G_UI: usize = 1;
const G_MINI: usize = 2;
const G_ART: usize = 3;
const G_FACE: usize = 4;
const G_BATTLE: usize = 5;
const G_MODELS: usize = 6;
const G_MAPS: usize = 7;
const G_VOICE: usize = 8;
const G_SOUND: usize = 9;
const G_VIDEO: usize = 10;
const G_OTHER: usize = 11;

const TEXT_PICS: [usize; 6] = [G_TEXT, G_UI, G_MINI, G_ART, G_FACE, G_BATTLE];

fn group_of(key: &str, text: bool) -> usize {
	if text { G_TEXT } else { crossbell_tool::crossbell::jobs::section_of(key) }
}

#[test]
fn groups_match_sections() {
	let names: Vec<&str> = groups().iter().map(|g| g.0).collect();
	assert_eq!(names, crossbell_tool::crossbell::jobs::SECTIONS);
}

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
	pub pics: bool,
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
	pending: Option<Arc<Mutex<Option<Vec<Item>>>>>,
	pub tex_dds: bool,
	pub tables_json: bool,
	pub other_files: bool,
	pub keep_failed: bool,
	page: u8,
	pub sections: bool,
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

fn folder_items(v: Vec<crossbell_tool::crossbell::jobs::Ed7Folder>, mode: Mode) -> Vec<Item> {
	let keys: std::collections::HashSet<String> = v.iter().map(|f| f.key.clone()).collect();
	let decomp = mode == Mode::Decompile;
	let mut out: Vec<Item> = v
		.into_iter()
		.filter(|f| f.scenes + f.tables + f.textures + f.sprites + f.others > 0)
		.map(|f| {
			let text = f.scenes + f.tables > 0;
			let group = group_of(&f.key, text);
			let us = f.key.ends_with("_us");
			let lang = if us || (f.scenes == 0 && f.tables > 0 && f.tables_us == f.tables) {
				tr("Английские файлы", "English files")
			} else if f.tables_us > 0 {
				tr("Японские и английские файлы", "Japanese and English files")
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
				let game = if f.dt2 == 0 { "._dt" } else if f.dt2 == f.tables { ".dt2" } else { "._dt / .dt2" };
				parts.push(format!("{} {}", plural(f.tables, tr("таблица", "table"), tr("таблицы", "tables"), tr("таблиц", "tables")), if decomp { format!("{game} → .xlsx") } else { format!(".xlsx → {game}") }));
			}
			if f.textures > 0 {
				parts.push(format!("{} {}", plural(f.textures, tr("картинка", "image"), tr("картинки", "images"), tr("картинок", "images")), if decomp { tr(".itp → .png или .dds", ".itp → .png or .dds") } else { ".png / .dds → .itp" }));
			}
			if f.sprites > 0 {
				parts.push(format!("{} {}", plural(f.sprites, tr("спрайт", "sprite"), tr("спрайта", "sprites"), tr("спрайтов", "sprites")), if decomp { tr(".itc → кадры .png", ".itc → .png frames") } else { tr("кадры .png → .itc", ".png frames → .itc") }));
			}
			if f.others > 0 {
				parts.push(lformat!("{} как есть", "{} as is", plural(f.others, tr("другой файл", "other file"), tr("других файла", "other files"), tr("других файлов", "other files"))));
			}
			let desc = if decomp { lformat!("Найдено: {}", "Found: {}", parts.join(", ")) } else { parts.join(", ") };
			let (right, ok) = if decomp {
				(fmt_size(f.size), false)
			} else if f.changed > 0 {
				(lformat!("изменено {}", "{} changed", f.changed), true)
			} else {
				(tr("без изменений", "no changes").to_string(), false)
			};
			Item { key: f.key.clone(), name: format!("{}\\", f.name), lang: lang.to_string(), desc, right, right_ok: ok, group, pics: f.textures + f.sprites > 0, on: true, size: f.size, unpacked: f.unpacked, changed: f.changed, out_key: f.key.clone(), out_size: f.out_size, mod_size: 0 }
		})
		.collect();
	out.sort_by_key(|i| (i.group, i.key.trim_end_matches("_us").to_string(), !i.key.ends_with("_us")));
	out
}

impl Picker {
	fn fmt_kinds(&self) -> (bool, bool) {
		(self.items.iter().any(|i| i.on && i.pics), false)
	}

	fn fmt_title(&self) -> &'static str {
		tr("Формат файлов", "File format")
	}

	pub fn open(mode: Mode, dir: &str, then_start: bool, ctx: &egui::Context) -> Picker {
		let mut p = Picker { mode, dir: dir.to_string(), items: Vec::new(), then_start, mod_mode: false, pending: None, tex_dds: false, tables_json: false, other_files: false, keep_failed: true, page: 0, sections: false };
		p.rescan(ctx);
		p
	}

	pub fn with_items(mode: Mode, dir: &str, items: Vec<Item>, then_start: bool) -> Picker {
		Picker { mode, dir: dir.to_string(), items, then_start, mod_mode: false, pending: None, tex_dds: false, tables_json: false, other_files: false, keep_failed: true, page: 0, sections: false }
	}

	pub fn rescan(&mut self, ctx: &egui::Context) {
		let slot = Arc::new(Mutex::new(None));
		self.pending = Some(slot.clone());
		self.items.clear();
		let dir = PathBuf::from(&self.dir);
		let mode = self.mode;
		let ctx = ctx.clone();
		std::thread::spawn(move || {
			let v = match mode {
				Mode::Decompile => crossbell_tool::crossbell::jobs::scan_game_ed7(&dir),
				Mode::Build => crossbell_tool::crossbell::jobs::scan_project_ed7(&dir),
			};
			*slot.lock().unwrap() = Some(folder_items(v, mode));
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
		self.items.iter().filter(|i| i.on).map(|i| i.out_size).sum()
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

		let fmt_page = decomp && self.page >= 1;
		let struct_page = decomp && self.page == 2;
			painter.text(Pos2::new(x0, y + 11.0), Align2::LEFT_CENTER, if struct_page { tr("Структура папок", "Folder structure") } else if fmt_page { self.fmt_title() } else if decomp { tr("Что разбирать", "What to decompile") } else { tr("Что собрать", "What to build") }, f_semi(19.0), p.text);
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
			let headline = if struct_page {
				tr("Как разложить разобранные файлы в папке проекта?", "How should the decompiled files be laid out in the project folder?").to_string()
			} else if fmt_page {
				tr("В каком виде разобрать файлы для правки и что делать с остальными?", "In what form to unpack the files for editing, and what to do with the rest?").to_string()
			} else if self.scanning() {
				if decomp { tr("Сканирую папку…", "Scanning the folder…").to_string() } else { tr("Смотрю папку проекта…", "Reading the project folder…").to_string() }
			} else if decomp {
				if n_all == 0 {
					tr("Подходящих папок не найдено — укажите папку игры или русификатора (в её data лежат scena_us, text_us…)", "No suitable folders found: choose the game folder or a translation mod's folder (its data folder holds scena_us, text_us…)").to_string()
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
				let text: &[usize] = &[G_TEXT];
				let tex: &[usize] = &TEXT_PICS;
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
				let (has_tex, has_tab) = self.fmt_kinds();
				let gap = 10.0;
				let cw = (list.width() - 30.0 - gap) / 2.0;
				let (card_h, line0, step) = (70.0, 36.0, 17.0);
				let mut cy = list.min.y + 6.0;
				let row = |ui: &mut egui::Ui, cy: &mut f32, title: &str, id: &str, cur: bool, opts: [(bool, &str, [&str; 2]); 2]| -> Option<bool> {
					painter.text(Pos2::new(list.min.x + 12.0, *cy + 8.0), Align2::LEFT_CENTER, title, f_semi(13.0), p.muted);
					*cy += 22.0;
					let mut pick = None;
					for (i, (v, t, lines)) in opts.iter().enumerate() {
						let r = Rect::from_min_size(Pos2::new(list.min.x + 12.0 + i as f32 * (cw + gap), *cy), Vec2::new(cw, card_h));
						if fmt_card(ui, &painter, p, r, (id, i), cur == *v, t, lines, line0, step) {
							pick = Some(*v);
						}
					}
					*cy += card_h + 10.0;
					pick
				};
				if struct_page {
					let sections = [(false, tr("Как в игре", "As in the game"), [tr("Папки игры прямо в папке проекта,", "The game's folders right in the project folder,"), tr("как раньше.", "as before.")]), (true, tr("По разделам", "By section"), [tr("Папки разделов программа создаст сама:", "The program creates the section folders itself:"), tr("«Текст для перевода», «Карты», «Портреты»…", "Text for translation, Maps, Portraits…")])];
					if let Some(v) = row(ui, &mut cy, tr("Папки проекта", "Project folders"), "structfmt", self.sections, sections) {
						self.sections = v;
					}
				} else {
				if has_tex {
					let dds = [tr("Те же блоки BC7, что в .itp, без потерь.", "The same BC7 blocks as in .itp, lossless."), tr("Нужен редактор с поддержкой DDS.", "Needs an editor that supports DDS.")];
					if let Some(v) = row(ui, &mut cy, tr("Текстуры", "Textures"), "texfmt", self.tex_dds, [(false, tr(".png — обычные картинки", ".png: ordinary images"), [tr("Любой редактор: Photoshop, GIMP, Krita.", "Any editor: Photoshop, GIMP, Krita."), tr("При сборке сожмутся обратно.", "They are compressed back when building.")]), (true, tr(".dds — как в игре", ".dds: as in the game"), dds)]) {
						self.tex_dds = v;
					}
				}
				if has_tab
					&& let Some(v) = row(ui, &mut cy, tr("Таблицы", "Tables"), "tabfmt", self.tables_json, [(false, ".xlsx — Excel", [tr("Только текст для перевода.", "Only the text to translate."), tr("Рядом оригинал.", "The original is next to it.")]), (true, tr(".json — все поля", ".json: all fields"), [tr("Текст и числа с названиями полей.", "Text and numbers with field names."), tr("Записи добавлять и удалять нельзя.", "Records cannot be added or removed.")])]) {
						self.tables_json = v;
					}
				if let Some(v) = row(ui, &mut cy, tr("Файлы без перевода (модели, сцены карт, звуки)", "Files without text (models, map scenes, sounds)"), "otherfmt", self.other_files, [(false, tr("Не класть в папку", "Leave them out"), [tr("Для перевода они не нужны.", "Not needed for translation."), tr("При сборке берутся из игры.", "The build takes them from the game.")]), (true, tr("Класть как есть", "Copy as they are"), [tr("Если хотите их менять.", "If you want to change them."), tr("Займут место на диске.", "They take disk space.")])]) {
					self.other_files = v;
				}
				if let Some(v) = row(ui, &mut cy, tr("Если файл не получится разобрать", "If a file cannot be unpacked"), "failfmt", self.keep_failed, [(true, tr("Положить как есть", "Copy it as it is"), [tr("Файл игры без изменений,", "The game file unchanged,"), tr("в журнале — предупреждение.", "with a warning in the log.")]), (false, tr("Пропустить", "Skip it"), [tr("Файла в папке не будет,", "The file is not in the folder,"), tr("только запись в журнале.", "only a line in the log.")])]) {
					self.keep_failed = v;
				}
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
			let (has_tex, has_tab) = self.fmt_kinds();
			let has_fmt = decomp;
			let (summary, hint) = if struct_page {
				let s = if self.sections { tr("По разделам — папки разделов создаются в папке проекта", "By section: section folders are created in the project folder") } else { tr("Как в игре — папки игры прямо в папке проекта", "As in the game: the game's folders right in the project folder") };
				(s.to_string(), tr("Уже разобранные папки остаются там, где лежат", "Folders that are already decompiled stay where they are").to_string())
			} else if fmt_page {
				let tex = lformat!("Текстуры — в {}", "Textures: as {}", if self.tex_dds { ".dds" } else { ".png" });
				let tab = lformat!("Таблицы — в {}", "Tables to {}", if self.tables_json { ".json" } else { ".xlsx" });
				let mut parts = Vec::new();
				if has_tex {
					parts.push(tex);
				}
				if has_tab {
					parts.push(tab);
				}
				parts.push(lformat!("без перевода — {}", "without text: {}", if self.other_files { tr("класть", "copy") } else { tr("не класть", "leave out") }));
				let s = parts.join(" · ");
				(s, tr("Выбор запоминается — в следующий раз будет отмечен он же", "The choice is remembered and will be ticked next time").to_string())
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
				let h = tr("Неотмеченные папки не собираются", "Unticked folders are not built").to_string();
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
			let next = has_fmt && self.page < 2;
			let ok_label = if next { tr("Далее", "Next") } else if decomp { tr("Готово", "Done") } else { tr("Собрать", "Build") };
			let ok_w = 120.0;
			let ok_r = Rect::from_min_size(Pos2::new(x1 - ok_w, foot.center().y - bh / 2.0), Vec2::new(ok_w, bh));
			let can = !self.scanning() && n_on > 0;
			let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(ok_r));
			let ok_icon = if next { "arrow-right" } else if decomp { "check" } else { "archive" };
			if primary_button_icon(&mut bu, p, ok_label, Some(ok_icon), ok_r.size(), 8, f_semi(14.5), can).clicked() {
				if next {
					self.page += 1;
				} else {
					ev = Event::Done;
				}
			}
			let cancel_r = Rect::from_min_max(Pos2::new(x0 + 300.0, ok_r.min.y), Pos2::new(ok_r.min.x - 10.0, ok_r.max.y));
			let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(cancel_r).layout(egui::Layout::right_to_left(egui::Align::Center)));
			if small_button(&mut cu, p, if fmt_page { tr("Назад", "Back") } else { tr("Отмена", "Cancel") }, Some(if fmt_page { "arrow-left" } else { "x" }), bh, true, p.text).clicked() {
				if fmt_page {
					self.page -= 1;
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

#[allow(clippy::too_many_arguments)]
fn fmt_card(ui: &mut egui::Ui, painter: &egui::Painter, p: &Pal, cr: Rect, id: (&str, usize), on: bool, title: &str, lines: &[&str; 2], line0: f32, step: f32) -> bool {
	let resp = ui.interact(cr, ui.id().with(id), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
	painter.rect(cr, 10, if on { p.sel.gamma_multiply(if p.dark { 0.6 } else { 0.55 }) } else if resp.hovered() { p.div2 } else { Color32::TRANSPARENT }, Stroke::new(if on { 1.5 } else { 1.0 }, if on { p.accent } else { p.div }), StrokeKind::Inside);
	let c = Pos2::new(cr.min.x + 26.0, cr.min.y + line0 - 18.0);
	painter.circle_stroke(c, 9.0, Stroke::new(1.5, if on { p.accent } else { p.muted }));
	if on {
		painter.circle_filled(c, 5.0, p.accent);
	}
	painter.text(Pos2::new(cr.min.x + 48.0, c.y), Align2::LEFT_CENTER, title, f_semi(15.0), p.text);
	for (j, l) in lines.iter().enumerate() {
		paint_text_clipped(ui, Rect::from_min_size(Pos2::new(cr.min.x + 48.0, cr.min.y + line0 + j as f32 * step), Vec2::new(cr.width() - 60.0, 18.0)), l, f_body(12.5), p.muted, Align2::LEFT_CENTER);
	}
	resp.clicked()
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
