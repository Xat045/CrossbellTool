#[allow(unused_imports)]
use crossbell_tool::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crossbell_tool::lang::tr;
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

pub const SENT0: u32 = 0xF0000;

pub const ITEM_BASE: u32 = 0x4000;
pub const RECORD_BASE: u32 = 0x6000;
const ED7_BTN_MIN: u32 = 160;

fn ed7() -> Option<crossbell_tool::crossbell::scena::Title> {
	crossbell_tool::crossbell::compare::ed7()
}

pub fn is_atlas(n: u32) -> bool {
	!(ED7_BTN_MIN..ITEM_BASE).contains(&n)
}

pub fn code_label(n: u32) -> String {
	if n >= RECORD_BASE { lformat!("достижение {}", "achievement {}", n - RECORD_BASE + 1) } else if n >= ITEM_BASE { lformat!("предмет {}", "item {}", n - ITEM_BASE) } else { format!("#{n}I") }
}

pub fn devices() -> [&'static str; 6] {
	devices_list()
}

pub fn devices_list() -> [&'static str; 6] {
	["PlayStation", "Xbox", "Nintendo Switch", "Steam Deck", "PlayStation 5", tr("Клавиатура", "Keyboard")]
}

pub fn pad_table() -> Vec<(&'static str, &'static str)> {
	vec![
	("south", tr("нижняя", "bottom")),
	("east", tr("правая", "right")),
	("west", tr("левая", "left")),
	("north", tr("верхняя", "top")),
	("L1", "L1"),
	("R1", "R1"),
	("L2", "L2"),
	("R2", "R2"),
	("L3", "L3"),
	("R3", "R3"),
	("LS", tr("левый стик", "left stick")),
	("RS", tr("правый стик", "right stick")),
	("up", tr("крест. ↑", "D-pad ↑")),
	("down", tr("крест. ↓", "D-pad ↓")),
	("left", tr("крест. ←", "D-pad ←")),
	("right", tr("крест. →", "D-pad →")),
	("dpadLR", tr("крест. ←→", "D-pad ←→")),
	("dpadUD", tr("крест. ↑↓", "D-pad ↑↓")),
	("dpad", tr("крестовина", "D-pad")),
	("view", "View / −"),
	("menu", "Menu / +"),
]
}

pub fn keys() -> Vec<(String, u32, u32)> {
	let mut v = Vec::new();
	let mut row = |r: u32, names: &[&str]| {
		for (i, n) in names.iter().enumerate() {
			if !n.is_empty() {
				v.push((n.to_string(), r, i as u32));
			}
		}
	};
	row(0, &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "^", "¥", "Q", "W", "E"]);
	row(1, &["R", "T", "Y", "U", "I", "O", "P", "@", "[", "A", "S", "D", "F", "G", "H", "J"]);
	row(2, &["K", "L", ";", ":", "]", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/", "\\"]);
	row(3, &["Del", "Fn", "Esc", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "Tab"]);
	row(4, &["Ctrl", "Ctrl R", "Alt", "Alt R", "Shift", "Shift R", "Backspace", "PrtScr", "Caps Lock", "Space", "Enter", "", "↑", "←", "↓", "→"]);
	row(5, &["Ins", "Home", "End", "Page Up", "Page Down", "Num Lock", "Num 1", "Num 2", "Num 3", "Num 4", "Num 5", "Num 6", "Num 7", "Num 8", "Num 9", "Num 0"]);
	row(6, &["Num /", "Num *", "Num -", "Num +", "Num .", "Num Enter", tr("Мышь Л", "Mouse L"), tr("Мышь П", "Mouse R"), tr("Колесо", "Wheel"), "WASD", tr("Стрелки", "Arrows"), "Num 2468", tr("Мышь", "Mouse"), tr("Колесо ↕", "Wheel ↕"), tr("Мышь колесо", "Mouse wheel")]);
	row(7, &["`", "", "", "", "", "", tr("Мышь 4", "Mouse 4"), tr("Мышь 5", "Mouse 5"), "", "", tr("Мышь ↕↔", "Mouse ↕↔")]);
	v
}

fn pad_cell(key: &str) -> Option<(u32, u32)> {
	Some(match key {
		"south" => (0, 0),
		"east" => (1, 0),
		"west" => (2, 0),
		"north" => (3, 0),
		"left" => (0, 1),
		"up" => (1, 1),
		"right" => (2, 1),
		"down" => (3, 1),
		"L1" => (0, 2),
		"R1" => (1, 2),
		"view" => (2, 2),
		"menu" => (3, 2),
		"LS" => (0, 3),
		"RS" => (1, 3),
		"dpad" => (3, 3),
		"L2" => (0, 4),
		"R2" => (1, 4),
		"dpadUD" => (2, 4),
		"dpadLR" => (3, 4),
		"L3" => (0, 5),
		"R3" => (1, 5),
		_ => return None,
	})
}
const SET_ROW: [u32; 5] = [0, 4, 8, 12, 0];

pub fn default_pad(n: u32) -> Option<&'static str> {
	let azure = ed7() == Some(crossbell_tool::crossbell::scena::Title::Azure);
	Some(match n {
		165 => "north",
		167 if azure => "L1",
		167 => "up",
		174 => "left",
		175 => "right",
		176 if !azure => "menu",
		192 if azure => "menu",
		194 if azure => "west",
		195 if azure => "south",
		_ => return None,
	})
}
fn default_kb(n: u32) -> Option<&'static str> {
	let azure = ed7() == Some(crossbell_tool::crossbell::scena::Title::Azure);
	Some(match n {
		165 => "R",
		167 if azure => "Q",
		167 => "W",
		174 => "A",
		175 => "D",
		176 if !azure => "Tab",
		192 if azure => "Tab",
		194 if azure => "I",
		195 if azure => tr("Мышь Л", "Mouse L"),
		_ => return None,
	})
}

pub fn action_name(n: u32) -> Option<&'static str> {
	let azure = ed7() == Some(crossbell_tool::crossbell::scena::Title::Azure);
	Some(match n {
		165 => tr("меню быстрого доступа", "quick access menu"),
		167 if azure => tr("быстрый доступ: особый слот (металлоискатель)", "quick access: special slot (metal detector)"),
		167 => tr("быстрый доступ ↑", "quick access ↑"),
		174 => tr("быстрый доступ ← (блокнот детектива)", "quick access ← (detective notebook)"),
		175 => tr("быстрый доступ → (боевой блокнот)", "quick access → (battle notebook)"),
		176 if !azure => tr("карта города / штата", "city / state map"),
		192 if azure => tr("карта города / штата", "city / state map"),
		194 if azure => tr("подробности (удерживать)", "details (hold)"),
		195 if azure => tr("подтвердить", "confirm"),
		_ => return None,
	})
}
pub struct Buttons {
	pub on: bool,
	pub device: usize,
	pub game_dir: String,
	pub pad: BTreeMap<u32, String>,
	pub kb: BTreeMap<u32, String>,
	icons: Option<image::RgbaImage>,
	prompts: Option<image::RgbaImage>,
	keys_img: Option<image::RgbaImage>,
	ps5_img: Option<image::RgbaImage>,
	record_img: Option<image::RgbaImage>,
	crafts: HashMap<String, u32>,
	items: HashMap<u32, u32>,
	loaded_from: String,
	cache: HashMap<String, Option<(TextureHandle, f32)>>,
	aspects: std::cell::RefCell<HashMap<String, f32>>,
	nonempty: std::cell::RefCell<HashMap<String, bool>>,
	pub status: String,
}

pub fn map_file() -> &'static str {
	"CrossbellCompare_buttons.txt"
}
pub const ED7_GAMES: [(&str, &str); 2] = [("Zero", "Zero"), ("Azure", "Azure")];

pub fn ed7_icons_dir() -> &'static str {
	if ed7() == Some(crossbell_tool::crossbell::scena::Title::Azure) { "Azure" } else { "Zero" }
}

pub use crossbell_tool::pack;

pub fn icons_root() -> PathBuf {
	crate::gui_common::exe_dir().join(tr("Значки", "Icons"))
}


impl Buttons {
	pub fn new(on: bool, device: usize, game_dir: String) -> Buttons {
		let mut b = Buttons { on, device: device.min(devices_list().len() - 1), game_dir, pad: BTreeMap::new(), kb: BTreeMap::new(), icons: None, prompts: None, keys_img: None, ps5_img: None, record_img: None, crafts: HashMap::new(), items: HashMap::new(), loaded_from: String::new(), cache: HashMap::new(), aspects: Default::default(), nonempty: Default::default(), status: String::new() };
		b.read_map();
		b
	}

	fn read_map(&mut self) {
		let dir = crate::gui_common::exe_dir();
		let (old, new) = (dir.join("KisekiCompare_buttons.txt"), dir.join(map_file()));
		if old.is_file() && !new.is_file() && std::fs::rename(&old, &new).is_err() {
			let _ = std::fs::copy(&old, &new);
		}
		let Ok(s) = std::fs::read_to_string(new) else { return };
		for l in s.lines() {
			let l = l.trim();
			if l.starts_with('#') || l.is_empty() {
				continue;
			}
			let Some((a, b)) = l.split_once('=') else { continue };
			let (a, b) = (a.trim(), b.trim().to_string());
			if let Some(n) = a.strip_suffix("kb").map(str::trim).and_then(|x| x.parse::<u32>().ok()) {
				self.kb.insert(n, b);
			} else if let Ok(n) = a.parse::<u32>() {
				self.pad.insert(n, b);
			}
		}
	}

	pub fn save_map(&self) {
		let mut s = String::from(tr("# Сверка: какая кнопка у кодов #160I… в тексте (только для показа в Сверке)\n# 165 = north  — кнопка геймпада по месту (south/east/west/north, L1, R1, L2, R2, L3, R3, LS, RS, up/down/left/right, dpad, view, menu)\n# 165 kb = R   — клавиша на клавиатуре\n", "# Compare: which button the #160I… codes in the text stand for (only for display in Compare)\n# 165 = north  : gamepad button by position (south/east/west/north, L1, R1, L2, R2, L3, R3, LS, RS, up/down/left/right, dpad, view, menu)\n# 165 kb = R   : keyboard key\n"));
		for (n, k) in &self.pad {
			s.push_str(&format!("{n} = {k}\n"));
		}
		for (n, k) in &self.kb {
			s.push_str(&format!("{n} kb = {k}\n"));
		}
		let _ = std::fs::write(crate::gui_common::exe_dir().join(map_file()), s);
	}

	pub fn ensure_loaded(&mut self) {
		if !ED7_GAMES.iter().any(|g| g.1 == self.game_dir.trim()) {
			let l = self.game_dir.to_lowercase();
			self.game_dir = if l.contains("azure") {
				"Azure"
			} else if l.contains("zero") {
				"Zero"
			} else {
				ed7_icons_dir()
			}
			.into();
		}
		let dir = self.game_dir.trim().to_string();
		if dir == self.loaded_from {
			return;
		}
		self.loaded_from = dir.clone();
		self.cache.clear();
		self.aspects.borrow_mut().clear();
		self.nonempty.borrow_mut().clear();
		self.icons = None;
		self.prompts = None;
		let d = icons_root().join(&dir);
		let in_pack = pack::has_game(&dir);
		let missing = !in_pack && !d.is_dir();
		let no_icons = lformat!("Нет файла «{}» рядом с программой — распакуйте архив Сверки целиком.", "The file '{}' is not next to the program: unpack the whole CrossbellCompare archive.", pack::FILE);
		self.load_set(&d, dir == "Azure", in_pack.then_some(dir.as_str()));
		if missing {
			self.status = no_icons;
		}
	}
	fn load_set(&mut self, d: &Path, azure: bool, pack: Option<&str>) {
		let rd = |name: &str| -> Option<Vec<u8>> {
			match pack {
				Some(g) => pack::get(&format!("{g}/{name}")).or_else(|| pack::get(name)).map(|b| b.to_vec()),
				None => std::fs::read(d.join(name)).or_else(|_| std::fs::read(d.parent().unwrap_or(d).join(name))).ok(),
			}
		};
		let load = |name: &str| rd(name).and_then(|b| image::load_from_memory(&b).ok()).map(|i| i.to_rgba8());
		self.icons = load("c_icon1.png");
		self.prompts = load("buttons.png");
		self.keys_img = load("keys.png");
		self.record_img = load("record.png");
		self.ps5_img = load("buttons_ps5.png");
		self.crafts.clear();
		if let Some(b) = rd("t_magic._dt") {
			self.crafts = parse_t_magic(&b);
		}
		self.items.clear();
		for f in ["t_item._dt", "t_item2._dt"] {
			if let Some(b) = rd(f) {
				for (k, v) in parse_t_item(&b) {
					self.items.entry(k).or_insert(v);
				}
			}
		}
		if let Some(b) = rd("t_quartz._dt") {
			{
				let (q, mq) = parse_t_quartz(&b, azure);
				let empty = |c: u32| self.icons.as_ref().map(|img| Self::trimmed(img, (c % 16) * 64, (c / 16) * 64, 64, 64).is_none()).unwrap_or(true);
				for (&id, &el) in &mq {
					self.items.insert(id, 73 + el.min(6));
				}
				let ids: Vec<u32> = self.items.keys().copied().collect();
				for id in ids {
					let cell = self.items[&id];
					if !empty(cell) {
						continue;
					}
					if let Some(&el) = mq.get(&id) {
						self.items.insert(id, 73 + el.min(6));
					} else if let Some(&el) = id.checked_sub(100).and_then(|q_id| q.get(&q_id)) {
						self.items.insert(id, 89 + el.min(6));
					}
				}
			}
		}
		let yes = |b: bool| if b { tr("найдена", "found") } else { tr("не найдена", "not found") };
		self.status = lformat!("c_icon1 (сепит, предметы) — {} · buttons/keys (кнопки, клавиши) — {} · t_item (значки предметов #…M) — {}", "c_icon1 (sepith, items) {} · buttons/keys (buttons, keys) {} · t_item (#…M item icons) {}",
			yes(self.icons.is_some()),
			yes(self.prompts.is_some() && self.keys_img.is_some()),
			if self.items.is_empty() { tr("не найдена", "not found").to_string() } else { lformat!("{} предметов", "{} items", self.items.len()) }
		);
	}

	pub fn craft_cell(&self, en: &str) -> Option<u32> {
		self.crafts.get(&craft_key(en)).copied()
	}

	pub fn drawable(&self, n: u32) -> bool {
		if is_atlas(n) {
			let Some((key, rects, src)) = self.locate(n) else { return false };
			if let Some(&v) = self.nonempty.borrow().get(&key) {
				return v;
			}
			let v = self.src_img(src).and_then(|img| Self::compose(img, &rects)).is_some();
			self.nonempty.borrow_mut().insert(key, v);
			v
		} else if self.device == 5 {
			self.prompts.is_some() && self.kb_of(n).is_some()
		} else {
			self.prompts.is_some() && self.pad_of(n).is_some()
		}
	}

	pub fn pad_of(&self, n: u32) -> Option<String> {
		self.pad.get(&n).cloned().or_else(|| default_pad(n).map(str::to_string))
	}

	pub fn is_custom(&self, n: u32) -> bool {
		if self.device == 5 { self.kb.contains_key(&n) } else { self.pad.contains_key(&n) }
	}

	pub fn kb_of(&self, n: u32) -> Option<String> {
		self.kb.get(&n).cloned().or_else(|| default_kb(n).map(str::to_string))
	}

	pub fn ready(&self) -> bool {
		self.icons.is_some() || self.prompts.is_some()
	}

	fn src_img(&self, src: u8) -> Option<&image::RgbaImage> {
		match src {
			0 => self.icons.as_ref(),
			2 => self.keys_img.as_ref(),
			3 => self.ps5_img.as_ref(),
			4 => self.record_img.as_ref(),
			_ => self.prompts.as_ref(),
		}
	}

	pub fn changed(&mut self) {
		self.cache.retain(|k, _| k.starts_with("i:"));
		self.aspects.borrow_mut().retain(|k, _| k.starts_with("i:"));
	}

	fn locate(&self, n: u32) -> Option<(String, Vec<(u32, u32, u32, u32)>, u8)> {
		if n >= RECORD_BASE {
			let c = n - RECORD_BASE;
			return Some((format!("r:{c}"), vec![((c % 8) * 128, (c / 8) * 128, 128, 128)], 4));
		}
		if is_atlas(n) {
			let c = if n >= ITEM_BASE { *self.items.get(&(n - ITEM_BASE))? } else { n };
			return Some((format!("i:{c}"), vec![((c % 16) * 64, (c / 16) * 64, 64, 64)], 0));
		}
		let key = if self.device == 5 { format!("k:{}", self.kb_of(n)?) } else { format!("p:{}:{}", self.device, self.pad_of(n)?) };
		let rects = Self::cells_of(&key)?;
		Some((key.clone(), rects, Self::key_src(&key)))
	}

	fn key_src(key: &str) -> u8 {
		if key.starts_with("k:") {
			2
		} else if key.starts_with("p:4:") {
			3
		} else {
			1
		}
	}

	fn cells_of(key: &str) -> Option<Vec<(u32, u32, u32, u32)>> {
		let (pre, body) = if let Some(b) = key.strip_prefix("k:") {
			("k:".to_string(), b)
		} else {
			let mut it = key.splitn(3, ':');
			let p = it.next()?;
			let d = it.next()?;
			(format!("{p}:{d}:"), it.next()?)
		};
		let mut v = Vec::new();
		for p in split_combo(body) {
			let (r, c) = Self::cell_pos(&format!("{pre}{p}"))?;
			v.push((c * 64, r * 64, 64, 64));
		}
		Some(v)
	}

	fn cell_pos(key: &str) -> Option<(u32, u32)> {
		if let Some(k) = key.strip_prefix("k:") {
			keys().into_iter().find(|x| x.0 == k).map(|x| (x.1, x.2))
		} else {
			let mut it = key.splitn(3, ':').skip(1);
			let dev: usize = it.next()?.parse().ok()?;
			let b = it.next()?;
			let row0 = if dev == 4 { 0 } else { SET_ROW[dev.min(4)] };
			pad_cell(b).map(|(r, c)| (row0 + r, c))
		}
	}

	pub fn aspect(&self, n: u32) -> f32 {
		let Some((key, rects, src)) = self.locate(n) else { return 1.0 };
		if let Some(a) = self.aspects.borrow().get(&key) {
			return *a;
		}
		let img = self.src_img(src);
		let a = img.and_then(|img| Self::compose(img, &rects)).map(|t| t.width() as f32 / t.height() as f32).unwrap_or(1.0);
		self.aspects.borrow_mut().insert(key, a);
		a
	}

	fn trimmed(img: &image::RgbaImage, x: u32, y: u32, w: u32, h: u32) -> Option<image::RgbaImage> {
		if x + w > img.width() || y + h > img.height() {
			return None;
		}
		let sub = image::imageops::crop_imm(img, x, y, w, h).to_image();
		let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
		for (px, py, p) in sub.enumerate_pixels() {
			if p[3] > 8 {
				x0 = x0.min(px);
				y0 = y0.min(py);
				x1 = x1.max(px + 1);
				y1 = y1.max(py + 1);
			}
		}
		if x1 <= x0 || y1 <= y0 {
			return None;
		}
		Some(image::imageops::crop_imm(&sub, x0, y0, x1 - x0, y1 - y0).to_image())
	}

	fn compose(img: &image::RgbaImage, rects: &[(u32, u32, u32, u32)]) -> Option<image::RgbaImage> {
		let parts: Vec<image::RgbaImage> = rects.iter().map(|&(x, y, w, h)| Self::trimmed(img, x, y, w, h)).collect::<Option<_>>()?;
		if parts.len() == 1 {
			return parts.into_iter().next();
		}
		let h = parts.iter().map(|p| p.height()).max()?;
		let plus = (h * 5 / 10).max(6);
		let gap = (h / 12).max(2);
		let w: u32 = parts.iter().map(|p| p.width()).sum::<u32>() + (parts.len() as u32 - 1) * (plus + 2 * gap);
		let mut out = image::RgbaImage::new(w, h);
		let mut x = 0;
		for (i, p) in parts.iter().enumerate() {
			if i > 0 {
				let cx = x + gap + plus / 2;
				let cy = h / 2;
				let t = (plus / 6).max(2);
				for yy in cy.saturating_sub(plus / 2)..(cy + plus / 2).min(h) {
					for xx in cx.saturating_sub(plus / 2)..(cx + plus / 2).min(w) {
						let on_v = xx + t / 2 >= cx && xx < cx + t - t / 2;
						let on_h = yy + t / 2 >= cy && yy < cy + t - t / 2;
						if on_v || on_h {
							out.put_pixel(xx, yy, image::Rgba([140, 140, 150, 255]));
						}
					}
				}
				x += plus + 2 * gap;
			}
			let dy = (h - p.height()) / 2;
			image::imageops::overlay(&mut out, p, x as i64, dy as i64);
			x += p.width();
		}
		Some(out)
	}

	fn upload_img(ctx: &egui::Context, name: &str, t: &image::RgbaImage) -> (TextureHandle, f32) {
		let ci = ColorImage::from_rgba_unmultiplied([t.width() as usize, t.height() as usize], t.as_raw());
		let tex = ctx.load_texture(name, ci, TextureOptions::LINEAR);
		(tex, t.width() as f32 / t.height() as f32)
	}

	pub fn icon(&mut self, ctx: &egui::Context, n: u32) -> Option<(TextureHandle, f32)> {
		let (key, rects, src) = self.locate(n)?;
		if let Some(v) = self.cache.get(&key) {
			return v.clone();
		}
		let img = self.src_img(src);
		let v = img.and_then(|img| Self::compose(img, &rects)).map(|t| Self::upload_img(ctx, &key, &t));
		self.cache.insert(key, v.clone());
		v
	}

	pub fn cell(&mut self, ctx: &egui::Context, key: &str) -> Option<(TextureHandle, f32)> {
		if let Some(v) = self.cache.get(key) {
			return v.clone();
		}
		let img = self.src_img(Self::key_src(key))?;
		let v = Self::cells_of(key).and_then(|r| Self::compose(img, &r)).map(|t| Self::upload_img(ctx, key, &t));
		self.cache.insert(key.to_string(), v.clone());
		v
	}

	pub fn label(&self, n: u32) -> Option<String> {
		if self.device == 5 {
			self.kb_of(n)
		} else {
			self.pad_of(n).map(|k| split_combo(&k).iter().map(|p| pad_table().iter().find(|x| x.0 == p).map(|x| x.1.to_string()).unwrap_or(p.clone())).collect::<Vec<_>>().join("+"))
		}
	}
}

pub fn split_combo(s: &str) -> Vec<String> {
	let mut v: Vec<String> = Vec::new();
	for seg in s.split('+') {
		if seg.trim().is_empty() {
			if let Some(last) = v.last_mut() {
				last.push('+');
			}
		} else {
			v.push(seg.trim().to_string());
		}
	}
	v.iter_mut().for_each(|x| {
		if x.ends_with('+') && !x.ends_with(" +") {
			x.insert(x.len() - 1, ' ');
		}
	});
	if v.is_empty() {
		v.push(s.to_string());
	}
	v
}

pub fn mark_codes(s: &str) -> String {
	let c: Vec<char> = s.chars().collect();
	let mut out = String::with_capacity(s.len());
	let mut i = 0;
	while i < c.len() {
		if c[i] == '{' && c[i..].iter().take(6).collect::<String>() == "{item[" {
			let mut j = i + 6;
			while j < c.len() && c[j].is_ascii_digit() {
				j += 1;
			}
			if j > i + 6 && c.get(j) == Some(&']') && c.get(j + 1) == Some(&'}') {
				let n: u32 = c[i + 6..j].iter().collect::<String>().parse().unwrap_or(u32::MAX);
				if let Some(ch) = (n < ITEM_BASE).then(|| char::from_u32(SENT0 + ITEM_BASE + n)).flatten() {
					out.push(ch);
					i = j + 2;
					continue;
				}
			}
		}
		if c[i] == '#' {
			let mut j = i + 1;
			while j < c.len() && c[j].is_ascii_digit() && j - i <= 5 {
				j += 1;
			}
			if j > i + 1 && j < c.len() && (c[j] == 'I' || c[j] == 'M') {
				let n: u32 = c[i + 1..j].iter().collect::<String>().parse().unwrap_or(0);
				let code = if c[j] == 'M' { ITEM_BASE + n } else { n };
				if n < ITEM_BASE
					&& let Some(ch) = char::from_u32(SENT0 + code) {
						out.push(ch);
						i = j + 1;
						continue;
					}
			}
		}
		out.push(c[i]);
		i += 1;
	}
	out
}
pub fn parse_t_item(b: &[u8]) -> HashMap<u32, u32> {
	let u16at = |o: usize| b.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]]) as usize);
	let mut m = HashMap::new();
	let Some(first) = u16at(0) else { return m };
	let groups = first / 2;
	let starts: Vec<usize> = (0..groups).filter_map(|g| u16at(g * 2)).collect();
	let Some(table_end) = starts.iter().filter_map(|&a| u16at(a)).filter(|&r| r >= first).min() else { return m };
	let mut recs = Vec::new();
	for g in 0..starts.len() {
		let e = starts.get(g + 1).copied().unwrap_or(table_end).min(table_end);
		let mut o = starts[g];
		while o + 2 <= e {
			if let Some(r) = u16at(o) {
				recs.push(r);
			}
			o += 2;
		}
	}
	for r in recs {
		if r < table_end || r + 6 > b.len() {
			continue;
		}
		let id = u16::from_le_bytes([b[r], b[r + 1]]) as u32;
		m.entry(id).or_insert(b[r + 5] as u32);
	}
	m
}

pub fn has_codes(s: &str) -> bool {
	(s.contains('#') && (s.contains('I') || s.contains('M'))) || s.contains("{item[")
}

pub fn record_cells(d: &crossbell_tool::crossbell::compare::Data) -> HashMap<usize, u32> {
	let mut rows: Vec<(u32, usize)> = d
		.pairs
		.iter()
		.enumerate()
		.filter(|(_, p)| p.file.rsplit('/').next().map(|f| f.starts_with("t_record")).unwrap_or(false))
		.filter_map(|(i, p)| p.id.strip_prefix('s').and_then(|x| x.parse::<u32>().ok()).map(|n| (n, i)))
		.collect();
	rows.sort();
	let text = |i: usize| d.pairs[i].en.as_ref().or(d.pairs[i].ru.as_ref()).map(|s| s.text.trim().to_string()).unwrap_or_default();
	let mut m = HashMap::new();
	let Some(sep) = rows.iter().position(|&(_, i)| { let t = text(i); t.is_empty() || t == "None" }) else { return m };
	for (k, &(_, i)) in rows.iter().enumerate() {
		let cell = if k < sep { k } else if k > sep && k - sep - 1 < sep { k - sep - 1 } else { continue };
		m.insert(i, cell as u32);
	}
	m
}

fn craft_key(s: &str) -> String {
	s.replace("\\n", "").chars().filter(|c| !c.is_whitespace()).collect()
}

pub fn parse_t_magic(b: &[u8]) -> HashMap<String, u32> {
	let u16at = |o: usize| b.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]]) as usize);
	let mut m = HashMap::new();
	let Some(first) = u16at(0) else { return m };
	let offs: Vec<usize> = (0..first / 2).filter_map(|i| u16at(i * 2)).collect();
	let Some(sz) = offs.get(1).zip(offs.first()).map(|(a, b)| a.saturating_sub(*b)).filter(|&s| (24..=64).contains(&s)) else { return m };
	let text = |o: usize| -> Option<String> {
		let e = b.get(o..)?.iter().position(|&c| c == 0)?;
		std::str::from_utf8(&b[o..o + e]).ok().map(|s| s.to_string())
	};
	for &o in &offs {
		if o + sz > b.len() {
			continue;
		}
		let cell = match u16at(o).unwrap_or(0) {
			7 => 44,
			8 => 42,
			4 if b[o + 8] == 100 || b[o + 9] == 100 => 47,
			4 => 46,
			_ => continue,
		};
		for f in [sz - 4, sz - 2] {
			if let Some(t) = u16at(o + f).and_then(text) {
				let k = craft_key(&t);
				if k.chars().count() > 1 {
					m.entry(k).or_insert(cell);
				}
			}
		}
	}
	m
}

pub fn parse_t_quartz(b: &[u8], azure: bool) -> (HashMap<u32, u32>, HashMap<u32, u32>) {
	let u16at = |o: usize| b.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]]) as u32);
	let (mut q, mut mq) = (HashMap::new(), HashMap::new());
	let (start, end) = if azure { (2, u16at(0).unwrap_or(0) as usize) } else { (0, b.len()) };
	let mut o = start;
	while o + 28 <= end.min(b.len()) {
		if let (Some(id), Some(el)) = (u16at(o), u16at(o + 2)) {
			q.insert(id, el.saturating_sub(1));
		}
		o += 28;
	}
	if azure {
		let mut o = end;
		while o + 44 <= b.len() {
			if let (Some(id), Some(el)) = (u16at(o), u16at(o + 2)) {
				mq.insert(id, el.saturating_sub(1));
			}
			o += 44;
		}
	}
	(q, mq)
}

pub fn code_of(c: char) -> Option<u32> {
	let v = c as u32;
	(SENT0..SENT0 + 0xFFFF).contains(&v).then(|| v - SENT0)
}

pub fn scan(texts: impl Iterator<Item = String>) -> BTreeMap<u32, (usize, String)> {
	let mut m: BTreeMap<u32, (usize, String)> = BTreeMap::new();
	for t in texts {
		let marked = mark_codes(&t);
		if !marked.chars().any(|c| code_of(c).is_some()) {
			continue;
		}
		let clean = crossbell_tool::crossbell::compare::clean(&marked, true);
		let chars: Vec<char> = clean.chars().collect();
		for (i, &c) in chars.iter().enumerate() {
			if let Some(n) = code_of(c) {
				let e = m.entry(n).or_insert((0, String::new()));
				e.0 += 1;
				if e.1.is_empty() {
					let a = i.saturating_sub(40);
					let b = (i + 40).min(chars.len());
					let s: String = chars[a..b].iter().map(|&c| if code_of(c) == Some(n) && c == chars[i] { '▮' } else if code_of(c).is_some() { '◦' } else if c == '\n' { ' ' } else { c }).collect();
					e.1 = format!("{}{}{}", if a > 0 { "…" } else { "" }, s.trim(), if b < chars.len() { "…" } else { "" });
				}
			}
		}
	}
	m
}
