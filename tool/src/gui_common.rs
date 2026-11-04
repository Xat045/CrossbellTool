#![allow(dead_code)]
#[allow(unused_imports)]
use crossbell_tool::{leprintln, lformat, lprintln};

use eframe::egui::{self, Align2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub use crossbell_tool::lang::{UI_EN, tr, ui_en};

pub struct Settings {
	map: BTreeMap<String, String>,
	path: PathBuf,
	title: &'static str,
}

impl Settings {
	pub fn load(file: &str, title: &'static str) -> Settings {
		let path = exe_dir().join(file);
		let mut map = BTreeMap::new();
		if let Ok(s) = std::fs::read_to_string(&path) {
			for l in s.trim_start_matches('\u{feff}').lines() {
				if let Some((k, v)) = l.split_once('=') {
					map.insert(k.trim().to_string(), v.trim().to_string());
				}
			}
		}
		Settings { map, path, title }
	}
	pub fn get(&self, k: &str) -> Option<&str> {
		self.map.get(k).map(|s| s.as_str())
	}
	pub fn get_or<'a>(&'a self, k: &str, d: &'a str) -> &'a str {
		self.get(k).unwrap_or(d)
	}
	pub fn set(&mut self, k: &str, v: &str) {
		self.map.insert(k.to_string(), v.to_string());
	}
	pub fn save(&self) {
		let mut s = lformat!("; настройки {}\r\n", "; {} settings\r\n", self.title);
		for (k, v) in &self.map {
			s.push_str(&format!("{k}={v}\r\n"));
		}
		let _ = std::fs::write(&self.path, s);
	}
}

pub fn exe_dir() -> PathBuf {
	std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_default()
}

#[derive(Clone, Copy)]
pub struct Pal {
	pub dark: bool,
	pub bg: Color32,
	pub surface: Color32,
	pub log: Color32,
	pub text: Color32,
	pub muted: Color32,
	pub div: Color32,
	pub div2: Color32,
	pub accent: Color32,
	pub accent_strong: Color32,
	pub on_accent: Color32,
	pub sel: Color32,
	pub hit: Color32,
	pub dict: Color32,
	pub ok: Color32,
	pub ok_strong: Color32,
	pub warn: Color32,
}

fn hex(v: u32) -> Color32 {
	Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

fn over(fg: Color32, bg: Color32, a: f32) -> Color32 {
	let m = |f: u8, b: u8| (f as f32 * a + b as f32 * (1.0 - a)).round() as u8;
	Color32::from_rgb(m(fg.r(), bg.r()), m(fg.g(), bg.g()), m(fg.b(), bg.b()))
}

impl Pal {
	pub fn light() -> Pal {
		let bg = hex(0xf2f2f3);
		let text = hex(0x1d1f20);
		Pal {
			dark: false,
			bg,
			surface: hex(0xe9e9ea),
			log: hex(0xebebec),
			text,
			muted: over(text, bg, 0.6),
			div: over(text, bg, 0.16),
			div2: over(text, bg, 0.08),
			accent: hex(0x5980a6),
			accent_strong: hex(0x416180),
			on_accent: hex(0xf2f2f3),
			sel: hex(0xd6ebff),
			hit: hex(0xb5d9fd),
			dict: hex(0xf9d68c),
			ok: hex(0xb3e4bd),
			ok_strong: hex(0x3f8a55),
			warn: hex(0xc0502e),
		}
	}
	pub fn graphite() -> Pal {
		let bg = hex(0x2a2c2f);
		let text = hex(0xd6d8da);
		Pal {
			dark: true,
			bg,
			surface: hex(0x323538),
			log: hex(0x26282b),
			text,
			muted: over(text, bg, 0.6),
			div: over(text, bg, 0.14),
			div2: over(text, bg, 0.07),
			accent: hex(0x8fb0d0),
			accent_strong: hex(0xaac6e2),
			on_accent: hex(0x1f2124),
			sel: hex(0x3a4a5c),
			hit: hex(0x3f5870),
			dict: hex(0xa07e45),
			ok: hex(0x2f4134),
			ok_strong: hex(0x8fd3a0),
			warn: hex(0xe8916f),
		}
	}
	pub fn get(dark: bool) -> Pal {
		if dark { Pal::graphite() } else { Pal::light() }
	}
}

pub fn apply_visuals(ctx: &egui::Context, p: &Pal) {
	let mut v = if p.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
	v.override_text_color = Some(p.text);
	v.panel_fill = p.bg;
	v.window_fill = p.bg;
	v.extreme_bg_color = p.surface;
	v.faint_bg_color = p.div2;
	v.window_stroke = Stroke::new(1.0, p.div);
	v.window_corner_radius = CornerRadius::same(10);
	v.menu_corner_radius = CornerRadius::same(10);
	v.selection.bg_fill = p.hit;
	v.selection.stroke = Stroke::new(1.0, p.text);
	v.hyperlink_color = p.accent;
	v.text_cursor.stroke = Stroke::new(1.5, p.text);
	for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
		w.corner_radius = CornerRadius::same(7);
		w.fg_stroke = Stroke::new(1.0, p.text);
	}
	v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.div);
	v.widgets.noninteractive.bg_fill = p.bg;
	v.widgets.inactive.bg_fill = p.surface;
	v.widgets.inactive.weak_bg_fill = p.surface;
	v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.div);
	v.widgets.hovered.bg_fill = p.div2;
	v.widgets.hovered.weak_bg_fill = p.sel;
	v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.div);
	v.widgets.active.bg_fill = p.sel;
	v.widgets.active.weak_bg_fill = p.sel;
	v.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
	v.widgets.open.bg_fill = p.surface;
	v.widgets.open.weak_bg_fill = p.surface;
	v.widgets.open.bg_stroke = Stroke::new(1.0, p.accent);
	v.popup_shadow = egui::epaint::Shadow { offset: [0, 12], blur: 32, spread: 0, color: Color32::from_black_alpha(if p.dark { 90 } else { 56 }) };
	v.window_shadow = v.popup_shadow;
	ctx.set_visuals(v);
	ctx.style_mut(|s| {
		s.spacing.item_spacing = Vec2::new(8.0, 6.0);
		s.spacing.button_padding = Vec2::new(10.0, 4.0);
		s.spacing.interact_size.y = 26.0;
		s.interaction.selectable_labels = false;
		s.text_styles.insert(egui::TextStyle::Body, f_body(14.0));
		s.text_styles.insert(egui::TextStyle::Button, f_body(13.0));
		s.text_styles.insert(egui::TextStyle::Small, f_body(11.5));
		s.text_styles.insert(egui::TextStyle::Monospace, f_mono(12.5));
		s.text_styles.insert(egui::TextStyle::Heading, f_semi(19.0));
	});
}

pub fn setup_fonts(ctx: &egui::Context) {
	let read = |names: &[&str]| -> Option<Vec<u8>> {
		let win = std::env::var("WINDIR").map(|w| PathBuf::from(w).join("Fonts")).unwrap_or_else(|_| PathBuf::from("C:\\Windows\\Fonts"));
		for n in names {
			let cands = [win.join(n), PathBuf::from(n)];
			for c in cands {
				if let Ok(b) = std::fs::read(&c) {
					return Some(b);
				}
			}
		}
		None
	};
	let mut fd = egui::FontDefinitions::default();
	let mut add = |key: &str, bytes: Option<Vec<u8>>| -> bool {
		match bytes {
			Some(b) => {
				fd.font_data.insert(key.to_string(), std::sync::Arc::new(egui::FontData::from_owned(b)));
				true
			}
			None => false,
		}
	};
	let lin = |f: &str| format!("/usr/share/fonts/truetype/{f}");
	let zlin = |f: &str| format!("Z:\\usr\\share\\fonts\\truetype\\{}", f.replace('/', "\\"));
	let body = add("body", read(&["segoeui.ttf", &lin("liberation/LiberationSans-Regular.ttf"), &zlin("liberation/LiberationSans-Regular.ttf"), &lin("dejavu/DejaVuSans.ttf")]));
	let semi = add("semi", read(&["seguisb.ttf", "segoeuib.ttf", &lin("liberation/LiberationSans-Bold.ttf"), &zlin("liberation/LiberationSans-Bold.ttf"), &lin("dejavu/DejaVuSans-Bold.ttf")]));
	let mono = add("mono", read(&["consola.ttf", &lin("dejavu/DejaVuSansMono.ttf"), &zlin("dejavu/DejaVuSansMono.ttf")]));
	let sym = add("sym", read(&["seguisym.ttf", &lin("dejavu/DejaVuSans.ttf"), &zlin("dejavu/DejaVuSans.ttf")]));
	let cjk = add("cjk", read(&["YuGothR.ttc", "meiryo.ttc", "msgothic.ttc", "/usr/share/fonts/opentype/ipafont-gothic/ipag.ttf", r"Z:\usr\share\fonts\opentype\ipafont-gothic\ipag.ttf"]));
	let zh = add("zh", read(&["msyh.ttc", "simsun.ttc", "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", r"Z:\usr\share\fonts\opentype\noto\NotoSansCJK-Regular.ttc"]));
	let prop = fd.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
	let monos = fd.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
	let mut body_fam = Vec::new();
	if body {
		body_fam.push("body".to_string());
	}
	if sym {
		body_fam.push("sym".to_string());
	}
	if cjk {
		body_fam.push("cjk".to_string());
	}
	if zh {
		body_fam.push("zh".to_string());
	}
	body_fam.extend(prop.iter().cloned());
	let mut semi_fam = Vec::new();
	if semi {
		semi_fam.push("semi".to_string());
	}
	semi_fam.extend(body_fam.iter().cloned());
	let mut mono_fam = Vec::new();
	if mono {
		mono_fam.push("mono".to_string());
	}
	mono_fam.extend(monos.iter().cloned());
	mono_fam.extend(body_fam.iter().cloned());
	fd.families.insert(FontFamily::Proportional, body_fam);
	fd.families.insert(FontFamily::Name("semi".into()), semi_fam);
	fd.families.insert(FontFamily::Monospace, mono_fam);
	ctx.set_fonts(fd);
}

pub fn f_body(size: f32) -> FontId {
	FontId::new(size, FontFamily::Proportional)
}
pub fn f_semi(size: f32) -> FontId {
	FontId::new(size, FontFamily::Name("semi".into()))
}
pub fn f_mono(size: f32) -> FontId {
	FontId::new(size, FontFamily::Monospace)
}

pub fn icon(name: &str) -> egui::ImageSource<'static> {
	let (uri, bytes): (&'static str, &'static [u8]) = match name {
		"archive" => ("bytes://archive.svg", include_bytes!("icons/archive.svg")),
		"package" => ("bytes://package.svg", include_bytes!("icons/package.svg")),
		"folder" => ("bytes://folder.svg", include_bytes!("icons/folder.svg")),
		"moon" => ("bytes://moon.svg", include_bytes!("icons/moon.svg")),
		"sun" => ("bytes://sun.svg", include_bytes!("icons/sun.svg")),
		"search" => ("bytes://search.svg", include_bytes!("icons/search.svg")),
		"folder-open" => ("bytes://folder-open.svg", include_bytes!("icons/folder-open.svg")),
		"download" => ("bytes://download.svg", include_bytes!("icons/download.svg")),
		"save" => ("bytes://save.svg", include_bytes!("icons/save.svg")),
		"file-plus" => ("bytes://file-plus.svg", include_bytes!("icons/file-plus.svg")),
		"file" => ("bytes://file.svg", include_bytes!("icons/file.svg")),
		"sheet" => ("bytes://sheet.svg", include_bytes!("icons/sheet.svg")),
		"undo" => ("bytes://undo.svg", include_bytes!("icons/undo.svg")),
		"stop" => ("bytes://stop.svg", include_bytes!("icons/stop.svg")),
		"x" => ("bytes://x.svg", include_bytes!("icons/x.svg")),
		"refresh" => ("bytes://refresh.svg", include_bytes!("icons/refresh.svg")),
		"info" => ("bytes://info.svg", include_bytes!("icons/info.svg")),
		"help" => ("bytes://help.svg", include_bytes!("icons/help.svg")),
		"languages" => ("bytes://languages.svg", include_bytes!("icons/languages.svg")),
		"arrow-left" => ("bytes://arrow-left.svg", include_bytes!("icons/arrow-left.svg")),
		"arrow-right" => ("bytes://arrow-right.svg", include_bytes!("icons/arrow-right.svg")),
		"switch" => ("bytes://switch.svg", include_bytes!("icons/switch.svg")),
		"heart" => ("bytes://heart.svg", include_bytes!("icons/heart.svg")),
		"list-plus" => ("bytes://list-plus.svg", include_bytes!("icons/list-plus.svg")),
		"chevron-down" => ("bytes://chevron-down.svg", include_bytes!("icons/chevron-down.svg")),
		"chevron-up" => ("bytes://chevron-up.svg", include_bytes!("icons/chevron-up.svg")),
		"eye-off" => ("bytes://eye-off.svg", include_bytes!("icons/eye-off.svg")),
		"external" => ("bytes://external.svg", include_bytes!("icons/external.svg")),
		"rotate-ccw" => ("bytes://rotate-ccw.svg", include_bytes!("icons/rotate-ccw.svg")),
		"eraser" => ("bytes://eraser.svg", include_bytes!("icons/eraser.svg")),
		"filter" => ("bytes://filter.svg", include_bytes!("icons/filter.svg")),
		"trash" => ("bytes://trash.svg", include_bytes!("icons/trash.svg")),
		"gamepad" => ("bytes://gamepad.svg", include_bytes!("icons/gamepad.svg")),
		"book" => ("bytes://book.svg", include_bytes!("icons/book.svg")),
		"columns" => ("bytes://columns.svg", include_bytes!("icons/columns.svg")),
		"wrench" => ("bytes://wrench.svg", include_bytes!("icons/wrench.svg")),
		_ => ("bytes://check.svg", include_bytes!("icons/check.svg")),
	};
	egui::ImageSource::Bytes { uri: uri.into(), bytes: egui::load::Bytes::Static(bytes) }
}

pub fn paint_icon(ui: &Ui, name: &str, rect: Rect, color: Color32) {
	let ppp = ui.ctx().pixels_per_point();
	let side = (rect.width().min(rect.height()) * ppp).round().max(1.0);
	let c = rect.center() * ppp;
	let min = Pos2::new((c.x - side / 2.0).round(), (c.y - side / 2.0).round());
	let rect = Rect::from_min_size(Pos2::new(min.x / ppp, min.y / ppp), Vec2::splat(side / ppp));
	let tex = icon_texture(ui.ctx(), name, side as u32);
	ui.painter().image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), color);
}

fn icon_texture(ctx: &egui::Context, name: &str, px: u32) -> egui::TextureHandle {
	let id = egui::Id::new(("icon_tex", name, px));
	if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
		return t;
	}
	let bytes: &[u8] = match icon(name) {
		egui::ImageSource::Bytes { bytes, .. } => match bytes {
			egui::load::Bytes::Static(b) => b,
			_ => &[],
		},
		_ => &[],
	};
	let img = egui_extras::image::load_svg_bytes_with_size(bytes, Some(egui::load::SizeHint::Size(px, px))).unwrap_or_else(|_| egui::ColorImage::new([1, 1], Color32::TRANSPARENT));
	let t = ctx.load_texture(format!("icon:{name}:{px}"), img, egui::TextureOptions::LINEAR);
	ctx.data_mut(|d| d.insert_temp(id, t.clone()));
	t
}

pub fn caps_job(text: &str, color: Color32) -> egui::text::LayoutJob {
	let mut job = egui::text::LayoutJob::default();
	job.append(&text.to_uppercase(), 0.0, egui::TextFormat { font_id: f_body(11.0), color, extra_letter_spacing: 0.9, ..Default::default() });
	job
}

pub fn paint_text_clipped(ui: &Ui, rect: Rect, text: &str, font: FontId, color: Color32, align: Align2) {
	let mut job = egui::text::LayoutJob::single_section(text.to_string(), egui::TextFormat { font_id: font, color, ..Default::default() });
	job.wrap = egui::text::TextWrapping { max_width: rect.width().max(1.0), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
	let g = ui.fonts(|f| f.layout_job(job));
	let pos = align.align_size_within_rect(g.size(), rect).min;
	ui.painter().with_clip_rect(rect).galley(pos, g, color);
}

pub fn path_input(ui: &mut Ui, p: &Pal, id: &str, value: &mut String, size: Vec2, hint: &str, enabled: bool) -> bool {
	let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
	ui.painter().rect(rect, 8, p.surface, Stroke::new(1.0, p.div), StrokeKind::Inside);
	let trect = rect.shrink2(Vec2::new(12.0, 0.0));
	let mut child = ui.new_child(egui::UiBuilder::new().max_rect(trect).layout(egui::Layout::left_to_right(egui::Align::Center)));
	let te = egui::TextEdit::singleline(value)
		.id(ui.id().with(id))
		.frame(false)
		.font(f_body(13.0))
		.hint_text(egui::RichText::new(hint).color(p.muted))
		.text_color(p.text)
		.desired_width(trect.width())
		.interactive(enabled)
		.margin(egui::Margin::ZERO);
	child.add(te).changed()
}

pub fn path_field(ui: &mut Ui, p: &Pal, id: &str, value: &mut String, height: f32, hint: &str, button: &str, highlight: bool, enabled: bool) -> (bool, bool, Rect) {
	let w = ui.available_width();
	let (rect, _) = ui.allocate_exact_size(Vec2::new(w, height), Sense::hover());
	let r = 8u8;
	let stroke = if highlight { p.accent } else { p.div };
	ui.painter().rect(rect, r, p.surface, Stroke::new(1.0, stroke), StrokeKind::Inside);
	let font = f_body(13.0);
	let tw = ui.fonts(|f| f.layout_no_wrap(button.to_string(), font.clone(), p.text).size().x);
	let bw = tw + 26.0 + 20.0;
	let brect = Rect::from_min_max(Pos2::new(rect.max.x - bw, rect.min.y), rect.max);
	let bresp = ui.interact(brect, ui.id().with(id).with("btn"), if enabled { Sense::click() } else { Sense::hover() });
	if bresp.hovered() && enabled {
		ui.painter().rect_filled(brect.shrink(1.0), CornerRadius { nw: 0, sw: 0, ne: r - 1, se: r - 1 }, p.div2);
	}
	ui.painter().line_segment([Pos2::new(brect.min.x, rect.min.y + 1.0), Pos2::new(brect.min.x, rect.max.y - 1.0)], Stroke::new(1.0, p.div));
	let bc = if enabled { p.text } else { p.muted };
	let bx = brect.min.x + 13.0;
	paint_icon(ui, "folder-open", Rect::from_center_size(Pos2::new(bx + 7.0, brect.center().y), Vec2::splat(14.0)), bc);
	ui.painter().text(Pos2::new(bx + 20.0, brect.center().y), Align2::LEFT_CENTER, button, font.clone(), bc);
	let trect = Rect::from_min_max(Pos2::new(rect.min.x + 12.0, rect.min.y), Pos2::new(brect.min.x - 6.0, rect.max.y));
	let mut child = ui.new_child(egui::UiBuilder::new().max_rect(trect).layout(egui::Layout::left_to_right(egui::Align::Center)));
	let te = egui::TextEdit::singleline(value)
		.id(ui.id().with(id))
		.frame(false)
		.font(font)
		.hint_text(egui::RichText::new(hint).color(p.muted))
		.text_color(p.text)
		.desired_width(trect.width())
		.interactive(enabled)
		.margin(egui::Margin::ZERO);
	let resp = child.add(te);
	(resp.changed(), bresp.clicked() && enabled, brect)
}

pub fn segmented(ui: &mut Ui, p: &Pal, id: &str, items: &[&str], sel: &mut usize, width: Option<f32>, height: f32, radius: u8, font: FontId, tips: &[&str]) -> bool {
	let pad = 14.0;
	let widths: Vec<f32> = items.iter().map(|t| ui.fonts(|f| f.layout_no_wrap(t.to_string(), font.clone(), p.text).size().x) + pad * 2.0).collect();
	let total = width.unwrap_or_else(|| widths.iter().sum());
	let equal = width.is_some();
	let (rect, _) = ui.allocate_exact_size(Vec2::new(total, height), Sense::hover());
	ui.painter().rect(rect, radius, Color32::TRANSPARENT, Stroke::new(1.0, p.div), StrokeKind::Inside);
	let mut x = rect.min.x;
	let mut changed = false;
	let n = items.len();
	for (i, t) in items.iter().enumerate() {
		let w = if equal { total / n as f32 } else { widths[i] };
		let r = Rect::from_min_size(Pos2::new(x, rect.min.y), Vec2::new(w, height));
		let mut resp = ui.interact(r, ui.id().with(id).with(i), Sense::click());
		if let Some(tip) = tips.get(i) {
			resp = resp.on_hover_text(*tip);
		}
		let cr = CornerRadius {
			nw: if i == 0 { radius } else { 0 },
			sw: if i == 0 { radius } else { 0 },
			ne: if i == n - 1 { radius } else { 0 },
			se: if i == n - 1 { radius } else { 0 },
		};
		if *sel == i {
			ui.painter().rect_filled(r, cr, p.accent);
		} else if resp.hovered() {
			ui.painter().rect_filled(r.shrink(1.0), cr, p.div2);
		}
		if i > 0 && *sel != i && *sel != i - 1 {
			ui.painter().line_segment([Pos2::new(x, rect.min.y + 1.0), Pos2::new(x, rect.max.y - 1.0)], Stroke::new(1.0, p.div));
		}
		ui.painter().text(r.center(), Align2::CENTER_CENTER, *t, font.clone(), if *sel == i { p.on_accent } else { p.text });
		if resp.clicked() && *sel != i {
			*sel = i;
			changed = true;
		}
		x += w;
	}
	changed
}

pub fn click_tip(ui: &Ui, resp: &Response, id: impl std::hash::Hash, add: impl FnOnce(&mut Ui)) {
	let pid = ui.id().with(("click_tip", id));
	let resp = resp.clone().on_hover_cursor(egui::CursorIcon::PointingHand);
	if resp.clicked() {
		ui.memory_mut(|m| m.toggle_popup(pid));
	}
	egui::popup_below_widget(ui, pid, &resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
		ui.set_min_width(220.0);
		add(ui);
	});
}

pub fn small_button(ui: &mut Ui, p: &Pal, text: &str, icon_name: Option<&str>, h: f32, enabled: bool, color: Color32) -> Response {
	let font = f_body(13.0);
	let tw = ui.fonts(|f| f.layout_no_wrap(text.to_string(), font.clone(), color).size().x);
	let iw = if icon_name.is_some() { 16.0 + 6.0 } else { 0.0 };
	let (rect, resp) = ui.allocate_exact_size(Vec2::new(tw + iw + 24.0, h), if enabled { Sense::click() } else { Sense::hover() });
	let fill = if resp.hovered() && enabled { p.div2 } else { Color32::TRANSPARENT };
	let alpha = if enabled { 1.0 } else { 0.4 };
	let c = color.gamma_multiply(alpha);
	ui.painter().rect(rect, 7, fill, Stroke::new(1.0, p.div.gamma_multiply(alpha)), StrokeKind::Inside);
	let mut x = rect.min.x + 12.0;
	if let Some(n) = icon_name {
		paint_icon(ui, n, Rect::from_center_size(Pos2::new(x + 8.0, rect.center().y), Vec2::splat(16.0)), c);
		x += iw;
	}
	ui.painter().text(Pos2::new(x, rect.center().y), Align2::LEFT_CENTER, text, font, c);
	if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp }
}

pub fn small_button_width(ui: &Ui, text: &str, icon: bool) -> f32 {
	ui.fonts(|f| f.layout_no_wrap(text.to_string(), f_body(13.0), Color32::WHITE).size().x) + if icon { 22.0 } else { 0.0 } + 24.0
}

pub fn primary_button(ui: &mut Ui, p: &Pal, text: &str, size: Vec2, radius: u8, font: FontId, enabled: bool) -> Response {
	primary_button_icon(ui, p, text, None, size, radius, font, enabled)
}

pub fn primary_button_icon(ui: &mut Ui, p: &Pal, text: &str, icon_name: Option<&str>, size: Vec2, radius: u8, font: FontId, enabled: bool) -> Response {
	let (rect, resp) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
	let fill = if !enabled { p.accent.gamma_multiply(0.55) } else if resp.hovered() { p.accent_strong } else { p.accent };
	ui.painter().rect_filled(rect, radius, fill);
	let is = (font.size + 1.0).round();
	let tw = ui.fonts(|f| f.layout_no_wrap(text.to_string(), font.clone(), p.on_accent).size().x);
	match icon_name {
		Some(n) if tw + is + 7.0 < size.x - 12.0 => {
			let x = rect.center().x - (tw + is + 7.0) / 2.0;
			paint_icon(ui, n, Rect::from_center_size(Pos2::new(x + is / 2.0, rect.center().y), Vec2::splat(is)), p.on_accent);
			ui.painter().text(Pos2::new(x + is + 7.0, rect.center().y), Align2::LEFT_CENTER, text, font, p.on_accent);
		}
		_ => {
			ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, font, p.on_accent);
		}
	}
	if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp }
}

pub fn checkbox(ui: &mut Ui, p: &Pal, value: &mut bool, text: &str, size: f32, font: FontId) -> Response {
	let tw = ui.fonts(|f| f.layout_no_wrap(text.to_string(), font.clone(), p.text).size().x);
	let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(size + 8.0 + tw, size.max(20.0)), Sense::click());
	let b = Rect::from_min_size(Pos2::new(rect.min.x, rect.center().y - size / 2.0), Vec2::splat(size));
	let rr = if size <= 13.0 { 3 } else { 4 };
	if *value {
		ui.painter().rect_filled(b, rr, p.accent);
		paint_icon(ui, "check", b.shrink(size * 0.12), p.on_accent);
	} else {
		ui.painter().rect(b, rr, Color32::TRANSPARENT, Stroke::new(1.0, if resp.hovered() { p.accent_strong } else { p.accent }), StrokeKind::Inside);
	}
	ui.painter().text(Pos2::new(b.max.x + 8.0, rect.center().y), Align2::LEFT_CENTER, text, font, p.text);
	if resp.clicked() {
		*value = !*value;
		resp.mark_changed();
	}
	resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn theme_button(ui: &mut Ui, p: &Pal, h: f32) -> Response {
	let (t, i) = if p.dark { (tr("Светлая тема", "Light theme"), "sun") } else { (tr("Тёмная тема", "Dark theme"), "moon") };
	small_button(ui, p, t, Some(i), h, true, p.text)
}

pub fn combo(ui: &mut Ui, p: &Pal, id: &str, items: &[String], sel: &mut usize, width: f32, height: f32, radius: u8, font: FontId) -> bool {
	let mut changed = false;
	let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
	let popup_id = ui.id().with(id).with("popup");
	let open = ui.memory(|m| m.is_popup_open(popup_id));
	ui.painter().rect(rect, radius, p.surface, Stroke::new(1.0, if open { p.accent } else { p.div }), StrokeKind::Inside);
	let label = items.get(*sel).cloned().unwrap_or_default();
	paint_text_clipped(ui, Rect::from_min_max(Pos2::new(rect.min.x + 12.0, rect.min.y), Pos2::new(rect.max.x - 28.0, rect.max.y)), &label, font.clone(), p.text, Align2::LEFT_CENTER);
	let c = Pos2::new(rect.max.x - 15.0, rect.center().y);
	ui.painter().add(egui::Shape::line(vec![Pos2::new(c.x - 4.0, c.y - 2.0), Pos2::new(c.x, c.y + 2.0), Pos2::new(c.x + 4.0, c.y - 2.0)], Stroke::new(1.5, p.text)));
	if resp.clicked() {
		ui.memory_mut(|m| m.toggle_popup(popup_id));
	}
	let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
	egui::popup::popup_below_widget(ui, popup_id, &resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
		ui.set_min_width(width - 12.0);
		egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
			for (i, t) in items.iter().enumerate() {
				let tw = ui.available_width();
				let (r, rr) = ui.allocate_exact_size(Vec2::new(tw, 28.0), Sense::click());
				if rr.hovered() || i == *sel {
					ui.painter().rect_filled(r, 7, if rr.hovered() { p.sel } else { p.div2 });
				}
				paint_text_clipped(ui, r.shrink2(Vec2::new(10.0, 0.0)), t, font.clone(), p.text, Align2::LEFT_CENTER);
				if rr.clicked() {
					if *sel != i {
						*sel = i;
						changed = true;
					}
				}
			}
		});
	});
	changed
}

#[derive(Clone)]
pub enum Dialog<A: Clone> {
	None,
	Info(String),
	Ask(String, A),
}

pub fn show_dialog<A: Clone>(ctx: &egui::Context, p: &Pal, title: &str, d: &mut Dialog<A>) -> Option<A> {
	let (text, ask) = match d {
		Dialog::None => return None,
		Dialog::Info(t) => (t.clone(), None),
		Dialog::Ask(t, a) => (t.clone(), Some(a.clone())),
	};
	let mut result = None;
	let mut close = false;
	let screen = ctx.screen_rect();
	egui::Area::new(egui::Id::new("dlg_dim")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
		let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
		ui.painter().rect_filled(r, 0, Color32::from_black_alpha(if p.dark { 120 } else { 70 }));
		let _ = resp;
	});
	egui::Area::new(egui::Id::new("dlg_box")).order(egui::Order::Foreground).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
		egui::Frame::new()
			.fill(p.bg)
			.stroke(Stroke::new(1.0, p.div))
			.corner_radius(12)
			.inner_margin(egui::Margin::same(20))
			.shadow(egui::epaint::Shadow { offset: [0, 12], blur: 32, spread: 0, color: Color32::from_black_alpha(64) })
			.show(ui, |ui| {
				ui.set_max_width(520.0);
				ui.label(egui::RichText::new(title).font(f_semi(18.0)).color(p.text));
				ui.add_space(8.0);
				ui.label(egui::RichText::new(&text).font(f_body(14.0)).color(p.text));
				ui.add_space(16.0);
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					if ask.is_some() {
						if primary_button_icon(ui, p, tr("Да", "Yes"), Some("check"), Vec2::new(96.0, 34.0), 8, f_semi(15.0), true).clicked() {
							result = ask.clone();
							close = true;
						}
						if small_button(ui, p, tr("Нет", "No"), Some("x"), 34.0, true, p.text).clicked() {
							close = true;
						}
					} else if primary_button_icon(ui, p, "OK", Some("check"), Vec2::new(96.0, 34.0), 8, f_semi(15.0), true).clicked() {
						close = true;
					}
				});
			});
	});
	if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
		close = true;
	}
	if ask.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
		close = true;
	}
	if close {
		*d = Dialog::None;
	}
	result
}

pub fn open_folder(path: &str) {
	let p = path.trim();
	if p.is_empty() {
		return;
	}
	let _ = std::fs::create_dir_all(p);
	#[cfg(windows)]
	let _ = std::process::Command::new("explorer").arg(p).spawn();
	#[cfg(not(windows))]
	let _ = std::process::Command::new("xdg-open").arg(p).spawn();
}

#[cfg(windows)]
static MAIN_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

pub fn set_dialog_owner(cc: &eframe::CreationContext) {
	#[cfg(windows)]
	{
		use raw_window_handle::{HasWindowHandle, RawWindowHandle};
		if let Ok(h) = cc.window_handle() {
			if let RawWindowHandle::Win32(w) = h.as_raw() {
				MAIN_HWND.store(w.hwnd.get(), std::sync::atomic::Ordering::Relaxed);
			}
		}
	}
	#[cfg(not(windows))]
	let _ = cc;
}

pub fn notify_done(title: &str, text: &str) {
	#[cfg(windows)]
	{
		use windows_sys::Win32::UI::Shell::{ExtractIconW, Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_TIP, NIIF_INFO, NIIF_LARGE_ICON, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW};
		use windows_sys::Win32::UI::WindowsAndMessaging::{FlashWindowEx, DestroyIcon, FLASHWINFO, FLASHW_TIMERNOFG, FLASHW_TRAY};
		let hwnd = MAIN_HWND.load(std::sync::atomic::Ordering::Relaxed);
		if hwnd == 0 {
			return;
		}
		let wide = |s: &str, out: &mut [u16]| {
			let n = out.len() - 1;
			for (d, c) in out.iter_mut().zip(s.encode_utf16().take(n)) {
				*d = c;
			}
		};
		let exe: Vec<u16> = std::env::current_exe().unwrap_or_default().as_os_str().encode_wide_null();
		let (title, text) = (title.to_string(), text.to_string());
		std::thread::spawn(move || unsafe {
			let mut nd: NOTIFYICONDATAW = std::mem::zeroed();
			nd.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
			nd.hWnd = hwnd as _;
			nd.uID = 0x4B54;
			nd.uFlags = NIF_ICON | NIF_TIP | NIF_INFO;
			nd.hIcon = ExtractIconW(std::ptr::null_mut(), exe.as_ptr(), 0);
			nd.dwInfoFlags = NIIF_INFO | NIIF_LARGE_ICON;
			wide(&title, &mut nd.szTip);
			wide(&title, &mut nd.szInfoTitle);
			wide(&text, &mut nd.szInfo);
			Shell_NotifyIconW(NIM_DELETE, &nd);
			Shell_NotifyIconW(NIM_ADD, &nd);
			let fl = FLASHWINFO { cbSize: std::mem::size_of::<FLASHWINFO>() as u32, hwnd: hwnd as _, dwFlags: FLASHW_TRAY | FLASHW_TIMERNOFG, uCount: 0, dwTimeout: 0 };
			FlashWindowEx(&fl);
			std::thread::sleep(std::time::Duration::from_secs(20));
			Shell_NotifyIconW(NIM_DELETE, &nd);
			if !nd.hIcon.is_null() {
				DestroyIcon(nd.hIcon);
			}
		});
	}
	#[cfg(not(windows))]
	let _ = (title, text);
}

#[cfg(windows)]
trait WideNull {
	fn encode_wide_null(&self) -> Vec<u16>;
}
#[cfg(windows)]
impl WideNull for std::ffi::OsStr {
	fn encode_wide_null(&self) -> Vec<u16> {
		use std::os::windows::ffi::OsStrExt;
		self.encode_wide().chain([0]).collect()
	}
}

#[cfg(windows)]
struct Owner(isize);
#[cfg(windows)]
impl raw_window_handle::HasWindowHandle for Owner {
	fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
		let h = std::num::NonZeroIsize::new(self.0).ok_or(raw_window_handle::HandleError::Unavailable)?;
		let raw = raw_window_handle::RawWindowHandle::Win32(raw_window_handle::Win32WindowHandle::new(h));
		Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(raw) })
	}
}
#[cfg(windows)]
impl raw_window_handle::HasDisplayHandle for Owner {
	fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
		Ok(raw_window_handle::DisplayHandle::windows())
	}
}
#[cfg(windows)]
fn with_owner(d: rfd::FileDialog) -> rfd::FileDialog {
	let h = MAIN_HWND.load(std::sync::atomic::Ordering::Relaxed);
	if h != 0 { d.set_parent(&Owner(h)) } else { d }
}

pub fn pick_folder(start: &str) -> Option<PathBuf> {
	#[cfg(windows)]
	{
		let mut d = with_owner(rfd::FileDialog::new());
		if std::path::Path::new(start).is_dir() {
			d = d.set_directory(start);
		}
		d.pick_folder()
	}
	#[cfg(not(windows))]
	{
		let _ = start;
		None
	}
}

pub fn pick_files(start: &str, title: &str, filter_name: &str, exts: &[&str], multi: bool) -> Vec<PathBuf> {
	#[cfg(windows)]
	{
		let mut d = with_owner(rfd::FileDialog::new()).set_title(title).add_filter(filter_name, exts).add_filter(tr("Все файлы", "All files"), &["*"]);
		if std::path::Path::new(start).is_dir() {
			d = d.set_directory(start);
		}
		if multi { d.pick_files().unwrap_or_default() } else { d.pick_file().into_iter().collect() }
	}
	#[cfg(not(windows))]
	{
		let _ = (start, title, filter_name, exts, multi);
		Vec::new()
	}
}

pub fn open_at_line(path: &std::path::Path, line: usize) -> Result<String, String> {
	if path.as_os_str().is_empty() || !path.is_file() {
		return Err(lformat!("Файл не найден: {}", "File not found: {}", path.display()));
	}
	let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
	if ext == "bin" || ext == "_dt" {
		return Err(tr("Это файл игры (.bin / ._dt) — открыть для правки нечего, правьте проект (.json / .xlsx)", "This is a game file (.bin / ._dt), there is nothing to open for editing: edit the project (.json / .xlsx)").into());
	}
	if ["json", "yaml", "yml", "txt"].contains(&ext.as_str()) {
		if let Some(npp) = notepad_pp() {
			std::process::Command::new(npp).arg(format!("-n{}", line.max(1))).arg(path).spawn().map_err(|e| lformat!("Notepad++ не запустился: {e}", "Notepad++ did not start: {e}"))?;
			return Ok(lformat!("Notepad++, строка {line}", "Notepad++, line {line}"));
		}
		if let Some(code) = vs_code() {
			let mut arg = path.as_os_str().to_os_string();
			arg.push(format!(":{}", line.max(1)));
			std::process::Command::new(code).arg("-g").arg(arg).spawn().map_err(|e| lformat!("VS Code не запустился: {e}", "VS Code did not start: {e}"))?;
			return Ok(lformat!("VS Code, строка {line}", "VS Code, line {line}"));
		}
	}
	#[cfg(windows)]
	if ext == "xlsx" && line > 0 {
		open_xlsx_at(path, line)?;
		return Ok(lformat!("Excel / WPS, лист «Text», строка {line}", "Excel / WPS, 'Text' sheet, line {line}"));
	}
	#[cfg(windows)]
	std::process::Command::new("explorer").arg(path).spawn().map_err(|e| e.to_string())?;
	#[cfg(not(windows))]
	std::process::Command::new("xdg-open").arg(path).spawn().map_err(|e| e.to_string())?;
	Ok(if line > 0 { lformat!("программой по умолчанию (нужная строка — {line})", "with the default program (the line you need is {line})") } else { tr("программой по умолчанию", "with the default program").into() })
}

#[cfg(windows)]
fn open_xlsx_at(path: &std::path::Path, row: usize) -> Result<(), String> {
	use std::os::windows::process::CommandExt;
	const SCRIPT: &str = r#"
$p = $env:KZ_OPEN_PATH; $row = [int]$env:KZ_OPEN_ROW
function Find-App { foreach ($id in 'Excel.Application', 'Ket.Application', 'et.Application') { try { return [Runtime.InteropServices.Marshal]::GetActiveObject($id) } catch {} }; $null }
function Find-Book($app) { if ($app) { foreach ($b in @($app.Workbooks)) { if ($b.FullName -ieq $p) { return $b } } }; $null }
$app = Find-App; $wb = Find-Book $app
if (-not $wb) {
	if ($app) { try { $wb = $app.Workbooks.Open($p) } catch {} }
	if (-not $wb) {
		Start-Process -FilePath $p
		for ($i = 0; $i -lt 60 -and -not $wb; $i++) { Start-Sleep -Milliseconds 500; $app = Find-App; $wb = Find-Book $app }
	}
}
if (-not $wb) { exit 1 }
$app.Visible = $true
$wb.Activate()
$ws = $null; try { $ws = $wb.Worksheets.Item('Text') } catch { $ws = $wb.Worksheets.Item(1) }
$ws.Activate()
$cell = $ws.Range("B$row")
try { $app.Goto($cell, $true) } catch {}
$cell.Select()
try { $app.ActiveWindow.ScrollRow = [Math]::Max(1, $row - 3) } catch {}
Add-Type -Name W -Namespace K -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h); [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);'
try { $h = [IntPtr]$app.Hwnd; [K.W]::ShowWindow($h, 9) | Out-Null; [K.W]::SetForegroundWindow($h) | Out-Null } catch {}
"#;
	let utf16: Vec<u8> = SCRIPT.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
	std::process::Command::new("powershell")
		.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &base64(&utf16)])
		.env("KZ_OPEN_PATH", path)
		.env("KZ_OPEN_ROW", row.to_string())
		.creation_flags(0x0800_0000)
		.spawn()
		.map(|_| ())
		.map_err(|e| lformat!("не удалось открыть таблицу: {e}", "could not open the table: {e}"))
}

#[cfg(windows)]
fn base64(b: &[u8]) -> String {
	const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
	let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
	for ch in b.chunks(3) {
		let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
		for k in 0..4 {
			s.push(if k <= ch.len() { T[(n >> (18 - 6 * k) & 63) as usize] as char } else { '=' });
		}
	}
	s
}

fn vs_code() -> Option<PathBuf> {
	let mut c = Vec::new();
	if let Some(d) = std::env::var_os("LOCALAPPDATA") {
		c.push(PathBuf::from(d).join("Programs").join("Microsoft VS Code").join("Code.exe"));
	}
	for v in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
		if let Some(d) = std::env::var_os(v) {
			c.push(PathBuf::from(d).join("Microsoft VS Code").join("Code.exe"));
		}
	}
	c.into_iter().find(|p| p.is_file())
}

fn notepad_pp() -> Option<PathBuf> {
	let mut cands: Vec<PathBuf> = Vec::new();
	for v in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
		if let Some(d) = std::env::var_os(v) {
			cands.push(PathBuf::from(d).join("Notepad++").join("notepad++.exe"));
		}
	}
	if let Some(c) = cands.into_iter().find(|p| p.is_file()) {
		return Some(c);
	}
	#[cfg(windows)]
	{
		use std::os::windows::ffi::{OsStrExt, OsStringExt};
		use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
		let key: Vec<u16> = std::ffi::OsStr::new(r"SOFTWARE\Notepad++").encode_wide().chain([0]).collect();
		let mut buf = vec![0u16; 1024];
		let mut size = (buf.len() * 2) as u32;
		let r = unsafe { RegGetValueW(HKEY_LOCAL_MACHINE, key.as_ptr(), std::ptr::null(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut size) };
		if r != 0 {
			return None;
		}
		let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
		let p = PathBuf::from(std::ffi::OsString::from_wide(&buf[..n])).join("notepad++.exe");
		return p.is_file().then_some(p);
	}
	#[cfg(not(windows))]
	None
}

pub fn save_file(start: &str, title: &str, name: &str, filter_name: &str, exts: &[&str]) -> Option<PathBuf> {
	#[cfg(windows)]
	{
		let mut d = with_owner(rfd::FileDialog::new()).set_title(title).set_file_name(name).add_filter(filter_name, exts);
		if std::path::Path::new(start).is_dir() {
			d = d.set_directory(start);
		}
		d.save_file()
	}
	#[cfg(not(windows))]
	{
		let _ = (start, title, name, filter_name, exts);
		None
	}
}

pub fn icon_from(png: &[u8]) -> egui::IconData {
	match image::load_from_memory(png) {
		Ok(im) => {
			let im = im.to_rgba8();
			egui::IconData { width: im.width(), height: im.height(), rgba: im.into_raw() }
		}
		Err(_) => window_icon(),
	}
}

pub fn window_icon() -> egui::IconData {
	let n = 32usize;
	let mut rgba = vec![0u8; n * n * 4];
	for y in 0..n {
		for x in 0..n {
			let inside = (3..29).contains(&x) && (3..29).contains(&y);
			let corner = |cx: f32, cy: f32| ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt() > 5.0;
			let cut = (x < 8 && y < 8 && corner(8.0, 8.0)) || (x >= 24 && y < 8 && corner(24.0, 8.0)) || (x < 8 && y >= 24 && corner(8.0, 24.0)) || (x >= 24 && y >= 24 && corner(24.0, 24.0));
			if inside && !cut {
				let i = (y * n + x) * 4;
				rgba[i..i + 4].copy_from_slice(&[0x59, 0x80, 0xa6, 255]);
			}
		}
	}
	egui::IconData { rgba, width: n as u32, height: n as u32 }
}
