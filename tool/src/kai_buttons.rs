#[allow(unused_imports)]
use crossbell_tool::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crossbell_tool::lang::tr;
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

pub const SENT0: u32 = 0xF0000;
pub const ATLAS_MAX: u32 = 441;

pub const ITEM_BASE: u32 = 0x4000;
pub const RECORD_BASE: u32 = 0x6000;
const ED7_BTN_MIN: u32 = 160;

fn ed7() -> Option<crossbell_tool::scena::Title> {
	crossbell_tool::kai::compare::ed7()
}

pub fn is_atlas(n: u32) -> bool {
	if ed7().is_some() { n < ED7_BTN_MIN || n >= ITEM_BASE } else { n < ATLAS_MAX }
}

pub fn code_label(n: u32) -> String {
	if ed7().is_some() {
		if n >= RECORD_BASE { lformat!("достижение {}", "achievement {}", n - RECORD_BASE + 1) } else if n >= ITEM_BASE { lformat!("предмет {}", "item {}", n - ITEM_BASE) } else { format!("#{n}I") }
	} else {
		format!("<I{n}>")
	}
}

pub fn devices() -> [&'static str; 6] {
	if ed7().is_some() { ["PlayStation", "Xbox", "Nintendo Switch", "Steam Deck", "PlayStation 5", tr("Клавиатура", "Keyboard")] } else { devices_list() }
}

pub fn devices_list() -> [&'static str; 6] {
	["PlayStation", "Xbox", "Nintendo Switch", "Steam Deck", "Stadia", tr("Клавиатура", "Keyboard")]
}
const SET_ROW: [u32; 5] = [0, 4, 12, 16, 8];

pub fn pad_table() -> Vec<(&'static str, &'static str, u32, u32)> {
	vec![
	("south", tr("нижняя", "bottom"), 0, 0),
	("east", tr("правая", "right"), 1, 0),
	("west", tr("левая", "left"), 2, 0),
	("north", tr("верхняя", "top"), 3, 0),
	("L1", "L1", 0, 4),
	("R1", "R1", 0, 5),
	("L2", "L2", 1, 4),
	("R2", "R2", 1, 5),
	("L3", "L3", 1, 6),
	("R3", "R3", 3, 6),
	("LS", tr("левый стик", "left stick"), 0, 6),
	("RS", tr("правый стик", "right stick"), 2, 6),
	("up", tr("крест. ↑", "D-pad ↑"), 2, 1),
	("down", tr("крест. ↓", "D-pad ↓"), 3, 1),
	("left", tr("крест. ←", "D-pad ←"), 0, 1),
	("right", tr("крест. →", "D-pad →"), 1, 1),
	("dpadLR", tr("крест. ←→", "D-pad ←→"), 1, 2),
	("dpadUD", tr("крест. ↑↓", "D-pad ↑↓"), 2, 2),
	("dpad", tr("крестовина", "D-pad"), 3, 2),
	("view", "View / −", 3, 4),
	("menu", "Menu / +", 3, 5),
]
}

pub fn keys() -> Vec<(String, u32, u32)> {
	if ed7().is_some() {
		return keys_ed7();
	}
	let mut v = Vec::new();
	let mut row = |r: u32, c0: u32, names: &[&str]| {
		for (i, n) in names.iter().enumerate() {
			if !n.is_empty() {
				v.push((n.to_string(), r, c0 + i as u32));
			}
		}
	};
	row(0, 18, &["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12"]);
	row(1, 18, &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]);
	row(2, 18, &["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"]);
	row(3, 18, &["A", "S", "D", "F", "G", "H", "J", "K", "L"]);
	row(4, 18, &["Z", "X", "C", "V", "B", "N", "M", "Space"]);
	row(5, 18, &["`", "-", "=", "[", "]", "\\", ";", "'", ",", ".", "/"]);
	row(6, 18, &["Shift", "Shift R", "Ctrl", "Ctrl R", "Alt", "Alt R", "Esc", "Enter", "Backspace", "Tab"]);
	row(7, 18, &["←", "→", "↑", "↓", "Ins", "Del", "Home", "End", "Page Up", "Page Down", "", tr("Стрелки", "Arrows")]);
	row(8, 18, &["Num 1", "Num 2", "Num 3", "Num 4", "Num 5", "Num 6", "Num 7", "Num 8", "Num 9", "Num 0"]);
	row(9, 18, &["Num +", "Num -", "Num *", "Num /", "Num Enter", "Num ."]);
	row(14, 18, &[tr("Мышь Л", "Mouse L"), tr("Мышь П", "Mouse R"), tr("Колесо", "Wheel"), tr("Мышь 4", "Mouse 4"), tr("Мышь 5", "Mouse 5"), tr("Колесо ↕", "Wheel ↕"), tr("Колесо ↑", "Wheel ↑"), tr("Колесо ↓", "Wheel ↓")]);
	v
}

fn keys_ed7() -> Vec<(String, u32, u32)> {
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

fn pad_cell_ed7(key: &str) -> Option<(u32, u32)> {
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
const SET_ROW_ED7: [u32; 5] = [0, 4, 8, 12, 0];

fn default_pad_ed7(n: u32) -> Option<&'static str> {
	let azure = ed7() == Some(crossbell_tool::scena::Title::Azure);
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
fn default_kb_ed7(n: u32) -> Option<&'static str> {
	let azure = ed7() == Some(crossbell_tool::scena::Title::Azure);
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

fn atlas_width(n: u32) -> u32 {
	match n {
		214 | 216 | 218 | 220 | 222 | 235 | 237 | 256 | 258 => 2,
		_ => 1,
	}
}

pub fn default_pad(n: u32) -> Option<&'static str> {
	if ed7().is_some() {
		return default_pad_ed7(n);
	}
	Some(match n {
		900 | 1062 | 1126 => "south",
		901 | 1127 => "east",
		1010 | 1060 | 1124 => "north",
		1011 | 1061 | 1125 => "west",
		1016 => "L1",
		1017 | 1128 => "R1",
		1018 => "L2",
		1019 => "R2",
		1024 => "R3",
		1063 | 1121 => "right",
		1064 | 1123 | 1159 => "left",
		1065 | 1120 | 1052 => "up",
		1068 | 1122 => "down",
		1129 | 1050 | 1066 => "menu",
		912 => "R3",
		907 => "L1",
		1114 => "L2",
		1051 => "view",
		1054 => "left",
		1055 => "right",
		1040 => "LS",
		1041 => "RS",
		1049 => "RS",
		1995 => "L1+R1",
		1996 => "dpadUD",
		906 => "R1",
		902 => "south",
		903 => "east",
		904 => "west",
		905 => "R2",
		908 => "north",
		909 => "R1",
		914 => "R3",
		1020 => "menu",
		1021 => "view",
		1031 => "L2",
		1053 => "down",
		1072 => "R3",
		1113 => "north",
		910 => "L2+R2",
		_ => return None,
	})
}

pub fn action_name(n: u32) -> Option<&'static str> {
	if ed7().is_some() {
		let azure = ed7() == Some(crossbell_tool::scena::Title::Azure);
		return Some(match n {
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
		});
	}
	Some(match n {
		900 => tr("подтвердить / говорить / осмотреть", "confirm / talk / examine"),
		901 => tr("отмена", "cancel"),
		902 => tr("атака на поле", "field attack"),
		903 => tr("уворот", "dodge"),
		904 => tr("развернуть осколки", "expand shards"),
		905 => tr("заряженная атака", "charged attack"),
		906 => tr("рывок", "dash"),
		907 => tr("сменить лидера", "change leader"),
		908 => tr("открыть сундук", "open chest"),
		909 => tr("автопродвижение текста", "text auto-advance"),
		910 => tr("ускоренный режим (L2+R2)", "high-speed mode (L2+R2)"),
		912 => tr("поиск осколков", "shard search"),
		914 => tr("захват цели", "target lock"),
		1010 => tr("действие меню (△)", "menu action (△)"),
		1011 => tr("действие меню (□)", "menu action (□)"),
		1016 => tr("вкладка влево", "tab left"),
		1017 => tr("вкладка вправо", "tab right"),
		1018 => tr("страница назад", "page back"),
		1019 => tr("страница вперёд", "page forward"),
		1020 => tr("пропуск", "skip"),
		1021 => tr("особое действие меню", "special menu action"),
		1024 => tr("прокрутка", "scroll"),
		1031 => tr("скрыть окно текста", "hide the text window"),
		1040 => tr("движение", "movement"),
		1041 => tr("камера", "camera"),
		1049 => tr("управление камерой (карта)", "camera control (map)"),
		1050 => tr("главное меню", "main menu"),
		1051 => tr("блокнот", "notebook"),
		1052 => tr("журнал", "log"),
		1053 => tr("развернуть миникарту", "expand the minimap"),
		1054 => tr("список дел", "to-do list"),
		1055 => tr("меню путешествий", "travel menu"),
		1060 => tr("лагерь: предметы", "camp: items"),
		1061 => tr("лагерь: орбмент", "camp: orbment"),
		1062 => tr("лагерь: снаряжение", "camp: equipment"),
		1063 => tr("лагерь: статус", "camp: status"),
		1064 => tr("лагерь: тактика", "camp: tactics"),
		1065 => tr("лагерь: достижения", "camp: achievements"),
		1068 => tr("лагерь: изготовление", "camp: crafting"),
		1113 => tr("быстрое искусство", "quick arts"),
		1114 => tr("Shortcut — удерживать L2", "Shortcut: hold L2"),
		1120 => tr("L2 + ↑: достижения", "L2 + ↑: achievements"),
		1121 => tr("L2 + →: отряд", "L2 + →: party"),
		1122 => tr("L2 + ↓: справка", "L2 + ↓: help"),
		1123 => tr("L2 + ←: термины", "L2 + ←: glossary"),
		1124 => tr("L2 + △: предметы", "L2 + △: items"),
		1125 => tr("L2 + □: лечение", "L2 + □: healing"),
		1126 => tr("L2 + ×: рыбалка", "L2 + ×: fishing"),
		1127 => "L2 + ○",
		1128 => tr("L2 + R1: магазин дронов", "L2 + R1: drone shop"),
		1129 => tr("L2 + Options: система", "L2 + Options: system"),
		1066 => tr("системное меню", "system menu"),
		1072 => tr("сменить позу", "change pose"),
		1159 => tr("термины к месту", "contextual glossary"),
		1161 | 1162 | 1163 => tr("ускоренный режим", "high-speed mode"),
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
	pub kai_items: HashMap<u32, u32>,
	pub kai_skills: HashMap<u32, u32>,
	pub kai_skill_rank: HashMap<u32, Vec<u32>>,
	pub kai_item_tail: HashMap<u32, Vec<u32>>,
	pub kai_ranges: HashMap<u32, u32>,
	pub kai_help: HashMap<String, u32>,
	kai_items_from: Option<PathBuf>,
	items: HashMap<u32, u32>,
	loaded_from: String,
	cache: HashMap<String, Option<(TextureHandle, f32)>>,
	aspects: std::cell::RefCell<HashMap<String, f32>>,
	nonempty: std::cell::RefCell<HashMap<String, bool>>,
	pub status: String,
}

pub fn map_file() -> &'static str {
	if crossbell_tool::kai::compare::ed7().is_some() { "KisekiCompare_buttons.txt" } else { "KaiCompare_buttons.txt" }
}

pub const ED7_GAMES: [(&str, &str); 2] = [("Zero", "Zero"), ("Azure", "Azure")];

pub fn ed7_icons_dir() -> &'static str {
	if ed7() == Some(crossbell_tool::scena::Title::Azure) { "Azure" } else { "Zero" }
}

pub use crossbell_tool::pack;

pub const KAI_GAMES: [(&str, &str); 3] = [("Daybreak 1", "Daybreak-1"), ("Daybreak 2", "Daybreak-2"), ("Horizon", "Horizon")];

pub fn icons_root() -> PathBuf {
	crate::gui_common::exe_dir().join(tr("Значки", "Icons"))
}


impl Buttons {
	pub fn new(on: bool, device: usize, game_dir: String) -> Buttons {
		let mut b = Buttons { on, device: device.min(devices_list().len() - 1), game_dir, pad: BTreeMap::new(), kb: BTreeMap::new(), icons: None, prompts: None, keys_img: None, ps5_img: None, record_img: None, crafts: HashMap::new(), kai_items: HashMap::new(), kai_skills: HashMap::new(), kai_skill_rank: HashMap::new(), kai_item_tail: HashMap::new(), kai_ranges: HashMap::new(), kai_help: HashMap::new(), kai_items_from: None, items: HashMap::new(), loaded_from: String::new(), cache: HashMap::new(), aspects: Default::default(), nonempty: Default::default(), status: String::new() };
		b.read_map();
		b
	}

	fn read_map(&mut self) {
		let Ok(s) = std::fs::read_to_string(crate::gui_common::exe_dir().join(map_file())) else { return };
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
		let mut s = String::from(tr("# Сверка: какая кнопка у кодов <I900…> в тексте (только для показа в Сверке)\n# 912 = R3     — кнопка геймпада по месту (south/east/west/north, L1, R1, L2, R2, L3, R3, LS, RS, up/down/left/right, dpad, view, menu)\n# 912 kb = Q   — клавиша на клавиатуре\n", "# Compare: which button the <I900…> codes in the text use (only for display in Compare)\n# 912 = R3     - a gamepad button by position (south/east/west/north, L1, R1, L2, R2, L3, R3, LS, RS, up/down/left/right, dpad, view, menu)\n# 912 kb = Q   - a keyboard key\n"));
		for (n, k) in &self.pad {
			s.push_str(&format!("{n} = {k}\n"));
		}
		for (n, k) in &self.kb {
			s.push_str(&format!("{n} kb = {k}\n"));
		}
		let _ = std::fs::write(crate::gui_common::exe_dir().join(map_file()), s);
	}

	pub fn ensure_loaded(&mut self) {
		if ed7().is_none() && !KAI_GAMES.iter().any(|g| g.1 == self.game_dir.trim()) {
			let l = self.game_dir.to_lowercase();
			self.game_dir = if ["daybreak-2", "daybreak 2", "daybreak ii", "daybreak2", "db2"].iter().any(|k| l.contains(k)) {
				"Daybreak-2"
			} else if l.contains("daybreak") || l.contains("db1") {
				"Daybreak-1"
			} else {
				"Horizon"
			}
			.into();
		}
		if ed7().is_some() && !ED7_GAMES.iter().any(|g| g.1 == self.game_dir.trim()) {
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
		if ed7().is_some() {
			self.load_ed7(&d, dir == "Azure", in_pack.then_some(dir.as_str()));
			if missing {
				self.status = no_icons;
			}
			return;
		}
		let file = |name: &str| -> Option<image::RgbaImage> {
			let b = if in_pack { pack::get(name)?.to_vec() } else { std::fs::read(icons_root().join(name)).ok()? };
			image::load_from_memory(&b).ok().map(|i| i.to_rgba8())
		};
		self.icons = file(&format!("{dir}/icons.png"));
		self.prompts = file("ph3_button_prompts.png");
		let yes = |b: bool| if b { tr("найдена", "found") } else { tr("не найдена", "not found") };
		self.status = if missing {
			no_icons
		} else {
			lformat!("icons.png (предметы, статусы, «!») — {} · ph3_button_prompts.png (кнопки устройств) — {}", "icons.png (items, statuses, '!') {} · ph3_button_prompts.png (device buttons) {}", yes(self.icons.is_some()), yes(self.prompts.is_some()))
		};
	}
	fn load_ed7(&mut self, d: &Path, azure: bool, pack: Option<&str>) {
		let rd = |name: &str| -> Option<Vec<u8>> {
			match pack {
				Some(g) => pack::get(&format!("{g}/{name}")).map(|b| b.to_vec()),
				None => std::fs::read(d.join(name)).ok(),
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

	pub fn load_kai_items(&mut self, places: &[PathBuf]) {
		let gd = PathBuf::from(self.game_dir.trim());
		let mut all: Vec<PathBuf> = places.to_vec();
		all.push(gd.clone());
		for base in all {
			for rel in ["_original/table_eng/t_item.tbl", "table_eng/t_item.tbl", "_original/table_en/t_item.tbl", "table_en/t_item.tbl", "_original/table/t_item.tbl", "t_item.tbl"] {
				let p = base.join(rel);
				if p.is_file() {
					if self.kai_items_from.as_ref() == Some(&p) {
						return;
					}
					if let Ok(b) = std::fs::read(&p) {
						self.kai_items = parse_kai_t_item(&b);
						self.kai_item_tail = parse_kai_item_tail(&b);
						if let Ok(sk) = std::fs::read(p.with_file_name("t_skill.tbl")) {
							self.kai_skills = parse_kai_t_skill(&sk);
							self.kai_skill_rank = parse_kai_skill_rank(&sk);
						}
						if let Ok(ih) = std::fs::read(p.with_file_name("t_itemhelp.tbl")) {
							self.kai_ranges = parse_kai_section_icon(&ih, "SkillRangeHelpData", 16);
							self.kai_help.clear();
							let mut put = |sec: &str, at: usize, fields: &[&str]| {
								for (k, v) in parse_kai_section_icon(&ih, sec, at) {
									for f in fields {
										self.kai_help.entry(format!("{sec}#{k}.{f}")).or_insert(v);
									}
								}
							};
							put("SkillEffectHelpData", 68, &["text3"]);
							put("SkillEffectHelpData", 64, &["text1", "text2", "text3"]);
							put("SkillItemStatusData", 32, &["text1", "text2"]);
							put("AttrTypeHelpData", 16, &["element_name"]);
						}

						self.kai_items_from = Some(p);
						return;
					}
				}
			}
		}
	}

	pub fn drawable(&self, n: u32) -> bool {
		if ed7().is_some() && is_atlas(n) {
			let Some((key, rects, src)) = self.locate(n) else { return false };
			if let Some(&v) = self.nonempty.borrow().get(&key) {
				return v;
			}
			let v = self.src_img(src).and_then(|img| Self::compose(img, &rects)).is_some();
			self.nonempty.borrow_mut().insert(key, v);
			v
		} else if is_atlas(n) {
			self.icons.is_some()
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
		self.kb.get(&n).cloned().or_else(|| if ed7().is_some() { default_kb_ed7(n).map(str::to_string) } else { None })
	}

	pub fn ready(&self) -> bool {
		self.icons.is_some() || self.prompts.is_some()
	}

	fn src_img(&self, src: u8) -> Option<&image::RgbaImage> {
		match src {
			0 => self.icons.as_ref(),
			2 if ed7().is_some() => self.keys_img.as_ref(),
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
		if ed7().is_some() {
			if n >= RECORD_BASE {
				let c = n - RECORD_BASE;
				return Some((format!("r:{c}"), vec![((c % 8) * 128, (c / 8) * 128, 128, 128)], 4));
			}
			if is_atlas(n) {
				let c = if n >= ITEM_BASE { *self.items.get(&(n - ITEM_BASE))? } else { n };
				return Some((format!("i:{c}"), vec![((c % 16) * 64, (c / 16) * 64, 64, 64)], 0));
			}
		} else if n < ATLAS_MAX {
			return Some((format!("i:{n}"), vec![((n % 21) * 48, (n / 21) * 48, 48 * atlas_width(n), 48)], 0));
		}
		let key = if self.device == 5 { format!("k:{}", self.kb_of(n)?) } else { format!("p:{}:{}", self.device, self.pad_of(n)?) };
		let rects = Self::cells_of(&key)?;
		Some((key.clone(), rects, Self::key_src(&key)))
	}

	fn key_src(key: &str) -> u8 {
		if key.starts_with("k:") {
			2
		} else if ed7().is_some() && key.starts_with("p:4:") {
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
			let sz = if ed7().is_some() { 64 } else { 128 };
			v.push((c * sz, r * sz, sz, sz));
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
			if ed7().is_some() {
				let row0 = if dev == 4 { 0 } else { SET_ROW_ED7[dev.min(4)] };
				return pad_cell_ed7(b).map(|(r, c)| (row0 + r, c));
			}
			pad_table().iter().find(|x| x.0 == b).map(|x| (SET_ROW[dev.min(4)] + x.2, x.3))
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
	if ed7().is_some() {
		return mark_codes_ed7(s);
	}
	let mut out = String::with_capacity(s.len());
	let mut rest = s;
	while let Some(p) = rest.find("<I") {
		out.push_str(&rest[..p]);
		let after = &rest[p + 2..];
		let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
		if !digits.is_empty() && after[digits.len()..].starts_with('>') {
			if let Some(ch) = digits.parse::<u32>().ok().filter(|&n| n < 0xFFFF).and_then(|n| char::from_u32(SENT0 + n)) {
				out.push(ch);
				rest = &after[digits.len() + 1..];
				continue;
			}
		}
		out.push_str("<I");
		rest = after;
	}
	out.push_str(rest);
	out
}

fn mark_codes_ed7(s: &str) -> String {
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
				if n < ITEM_BASE {
					if let Some(ch) = char::from_u32(SENT0 + code) {
						out.push(ch);
						i = j + 1;
						continue;
					}
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
	if ed7().is_some() { (s.contains('#') && (s.contains('I') || s.contains('M'))) || s.contains("{item[") } else { s.contains("<I") }
}

pub fn record_cells(d: &crossbell_tool::kai::compare::Data) -> HashMap<usize, u32> {
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

pub fn parse_kai_t_item(d: &[u8]) -> HashMap<u32, u32> {
	let mut m = HashMap::new();
	let u32at = |o: usize| d.get(o..o + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize);
	let u16at = |o: usize| d.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]]) as u32);
	if d.get(0..4) != Some(b"#TBL") {
		return m;
	}
	let n = u32at(4).unwrap_or(0);
	for i in 0..n {
		let h = 8 + i * 80;
		let Some(name) = d.get(h..h + 64) else { break };
		let name = String::from_utf8_lossy(name.split(|&c| c == 0).next().unwrap_or(&[])).to_string();
		if name != "ItemTableData" {
			continue;
		}
		let (Some(off), Some(sz), Some(cnt)) = (u32at(h + 68), u32at(h + 72), u32at(h + 76)) else { break };
		if sz < 28 {
			break;
		}
		for k in 0..cnt {
			let r = off + k * sz;
			let (Some(chr), Some(cat), Some(icon)) = (u16at(r + 4), d.get(r + 24).copied(), u16at(r + 26)) else { break };
			let cell = if icon != 0 {
				icon
			} else {
				match cat {
					0 => match u16at(r) {
						Some(id @ 310..=317) => 112 + id - 310,
						_ => continue,
					},
					12 => match chr {
						0 => 22,
						1 => 23,
						9 => 34,
						149 => 41,
						2 => 24,
						3 => 25,
						4 => 27,
						5 => 29,
						6 => 26,
						8 => 40,
						10 => 28,
						11 => 30,
						12 => 33,
						17 => 36,
						18 => 37,
						19 => 35,
						20 => 53,
						21 => 54,
						22 => 55,
						23 => 56,
						24 => 57,
						62 => 32,
						63 => 31,
						151 => 38,
						152 => 39,
						159 => 58,
						_ => continue,
					},
					13 => 21,
					14 => 2,
					15 => 3,
					16 => 4,
					17 => 15,
					18 => 17,
					19 => 16,
					23 => match d.get(r + 30).copied() {
						Some(e @ 1..=7) => 41 + e as u32,
						_ => continue,
					},
					24 => 13,
					25 => 5,
					_ => continue,
				}
			};
			m.insert(k as u32, cell);
		}
	}
	m
}

pub fn parse_kai_t_skill(d: &[u8]) -> HashMap<u32, u32> {
	let mut m = HashMap::new();
	let u32at = |o: usize| d.get(o..o + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize);
	if d.get(0..4) != Some(b"#TBL") {
		return m;
	}
	let n = u32at(4).unwrap_or(0);
	for i in 0..n {
		let h = 8 + i * 80;
		let Some(name) = d.get(h..h + 64) else { break };
		if name.split(|&c| c == 0).next() != Some(b"SkillParam".as_slice()) {
			continue;
		}
		let (Some(off), Some(sz), Some(cnt)) = (u32at(h + 68), u32at(h + 72), u32at(h + 76)) else { break };
		for k in 0..cnt {
			let r = off + k * sz;
			let Some(kind) = d.get(r + 16).copied() else { break };
			let flags: Vec<u8> = d.get(r + 8..r + 16).map(|x| u64::from_le_bytes(x.try_into().unwrap()) as usize).and_then(|p| d.get(p..)).map(|x| x.iter().take_while(|&&c| c != 0).copied().collect()).unwrap_or_default();
			let cell = match kind {
				3 => 293,
				2 if flags.contains(&b'Z') => 292,
				2 => 290,
				_ => continue,
			};
			m.insert(k as u32, cell);
		}
	}
	m
}

pub fn parse_kai_section_icon(d: &[u8], section: &str, at: usize) -> HashMap<u32, u32> {
	let mut m = HashMap::new();
	let u32at = |o: usize| d.get(o..o + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize);
	if d.get(0..4) != Some(b"#TBL") {
		return m;
	}
	for i in 0..u32at(4).unwrap_or(0) {
		let h = 8 + i * 80;
		let Some(name) = d.get(h..h + 64) else { break };
		if name.split(|&c| c == 0).next() != Some(section.as_bytes()) {
			continue;
		}
		let (Some(off), Some(sz), Some(cnt)) = (u32at(h + 68), u32at(h + 72), u32at(h + 76)) else { break };
		for k in 0..cnt {
			if let Some(v) = d.get(off + k * sz + at..off + k * sz + at + 2).map(|x| u16::from_le_bytes([x[0], x[1]]) as u32) {
				if v > 0 && v < ATLAS_MAX {
					m.insert(k as u32, v);
				}
			}
		}
	}
	m
}


pub fn parse_kai_skill_rank(d: &[u8]) -> HashMap<u32, Vec<u32>> {
	let mut m = HashMap::new();
	let u32at = |o: usize| d.get(o..o + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize);
	if d.get(0..4) != Some(b"#TBL") {
		return m;
	}
	let n = u32at(4).unwrap_or(0);
	let mut sec: HashMap<String, (usize, usize, usize)> = HashMap::new();
	for i in 0..n {
		let h = 8 + i * 80;
		let Some(name) = d.get(h..h + 64) else { break };
		let name = String::from_utf8_lossy(name.split(|&c| c == 0).next().unwrap_or(&[])).to_string();
		if let (Some(o), Some(sz), Some(c)) = (u32at(h + 68), u32at(h + 72), u32at(h + 76)) {
			sec.insert(name, (o, sz, c));
		}
	}
	let Some(&(so, ssz, sc)) = sec.get("SkillParam") else { return m };
	let mut th: Vec<(usize, u32)> = sec.get("SkillPowerIcon").map(|&(po, psz, pc)| (0..pc).filter_map(|k| Some((u32at(po + k * psz)?, u32at(po + k * psz + 4)? as u32))).collect()).unwrap_or_default();
	th.sort();
	for k in 0..sc {
		let r = so + k * ssz;
		let Some(kind) = d.get(r + 16).copied() else { break };
		if kind != 2 && kind != 3 {
			continue;
		}
		let mut v = Vec::new();
		if let Some(e) = d.get(r + 142..r + 144).map(|x| u16::from_le_bytes([x[0], x[1]]) as u32) {
			if e > 0 && e < ATLAS_MAX {
				v.push(e);
			}
		}
		let flags: Vec<u8> = d.get(r + 8..r + 16).map(|x| u64::from_le_bytes(x.try_into().unwrap()) as usize).and_then(|p| d.get(p..)).map(|x| x.iter().take_while(|&&c| c != 0).copied().collect()).unwrap_or_default();
		v.extend(kai_range_icon(d.get(r + 32).copied().unwrap_or(0), &flags));
		if let Some(p) = d.get(r + 52..r + 54).map(|x| u16::from_le_bytes([x[0], x[1]]) as usize).filter(|&p| p > 0) {
			if let Some(&(_, icon)) = th.iter().rev().find(|(t, _)| *t <= p).or(th.first()) {
				v.push(icon);
			}
		}
		if !v.is_empty() {
			m.insert(k as u32, v);
		}
	}
	m
}

fn kai_range_icon(shape: u8, flags: &[u8]) -> Option<u32> {
	let ally = flags.contains(&b'F') || !flags.contains(&b'E');
	if shape & 0x80 != 0 {
		Some(if ally { 301 } else { 298 })
	} else if shape & 0x10 != 0 {
		Some(296)
	} else if shape & 0x20 != 0 {
		Some(if ally { 300 } else { 295 })
	} else if shape & 0x09 != 0 {
		Some(if ally { 299 } else { 294 })
	} else {
		None
	}
}

pub fn parse_kai_item_tail(d: &[u8]) -> HashMap<u32, Vec<u32>> {
	let mut m = HashMap::new();
	let u32at = |o: usize| d.get(o..o + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize);
	if d.get(0..4) != Some(b"#TBL") {
		return m;
	}
	for i in 0..u32at(4).unwrap_or(0) {
		let h = 8 + i * 80;
		let Some(name) = d.get(h..h + 64) else { break };
		if name.split(|&c| c == 0).next() != Some(b"ItemTableData".as_slice()) {
			continue;
		}
		let (Some(off), Some(sz), Some(cnt)) = (u32at(h + 68), u32at(h + 72), u32at(h + 76)) else { break };
		if sz < 40 {
			break;
		}
		for k in 0..cnt {
			let r = off + k * sz;
			let Some(e) = d.get(r + 28..r + 30).map(|x| u16::from_le_bytes([x[0], x[1]]) as u32) else { break };
			if e == 0 || e >= ATLAS_MAX {
				continue;
			}
			let flags: Vec<u8> = d.get(r + 8..r + 16).map(|x| u64::from_le_bytes(x.try_into().unwrap()) as usize).and_then(|p| d.get(p..)).map(|x| x.iter().take_while(|&&c| c != 0).copied().collect()).unwrap_or_default();
			let mut v = vec![e];
			v.extend(kai_range_icon(d.get(r + 32).copied().unwrap_or(0), &flags));
			m.insert(k as u32, v);
		}
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
	let Some(sz) = offs.get(1).zip(offs.first()).map(|(a, b)| a.saturating_sub(*b)).filter(|&s| s >= 24 && s <= 64) else { return m };
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

pub fn ui_icon(file: &str, id: &str) -> Option<u32> {
	let f = file.rsplit('/').next().unwrap_or(file);
	if f.starts_with("t_text.") {
		let n: u32 = id.strip_prefix("TextTableData#")?.strip_suffix(".value")?.parse().ok()?;
		return Some(match n {
			955 => 405,
			956 => 406,
			958 => 426,
			962 => 409,
			957 => 408,
			959 => 427,
			960 => 428,
			961 => 429,
			963 => 430,
			964 => 410,
			981 => 1010,
			982 => 1050,
			985 => 1018,
			989 => 1995,
			990 => 1024,
			991 => 901,
			803 => 1011,
			804 | 808 => 1996,
			805 => 900,
			_ => return None,
		});
	}
	if f.starts_with("t_dlc.") && id.starts_with("DLCTableData#") && id.ends_with(".text1") {
		return Some(6);
	}
	if f.starts_with("t_free_dungeon.") {
		if let Some(r) = id.strip_prefix("FreeDungeonMassEffectText#") {
			return match r.split('.').next()? {
				"0" => Some(183),
				"1" => Some(184),
				"2" => Some(185),
				"3" => Some(186),
				"4" => Some(188),
				"5" => Some(182),
				"6" => Some(406),
				"7" => Some(181),
				"8" => Some(187),
				"13" => Some(180),
				"14" => Some(121),
				_ => None,
			};
		}

	}
	None
}


const COND_HZ: [u32; 68] = [322, 316, 315, 317, 318, 320, 325, 319, 321, 326, 309, 0, 332, 333, 334, 0, 359, 314, 365, 382, 383, 384, 335, 312, 313, 324, 310, 388, 386, 387, 311, 356, 356, 357, 327, 328, 329, 385, 0, 264, 348, 349, 350, 351, 336, 338, 340, 342, 344, 346, 352, 353, 354, 355, 337, 339, 341, 343, 345, 347, 330, 331, 361, 362, 363, 364, 250, 251];
const COND_D1: [u32; 54] = [322, 316, 315, 317, 318, 320, 325, 319, 321, 326, 309, 0, 332, 333, 334, 0, 359, 314, 335, 312, 313, 324, 310, 311, 356, 327, 328, 329, 348, 349, 350, 351, 336, 338, 340, 342, 344, 346, 352, 353, 354, 355, 337, 339, 341, 343, 345, 347, 330, 331, 361, 362, 363, 364];
const COND_D2: [u32; 59] = [322, 316, 315, 317, 318, 320, 325, 319, 321, 326, 309, 0, 332, 333, 334, 0, 359, 314, 365, 335, 312, 313, 324, 310, 388, 386, 387, 311, 356, 357, 327, 328, 329, 348, 349, 350, 351, 336, 338, 340, 342, 344, 346, 352, 353, 354, 355, 337, 339, 341, 343, 345, 347, 330, 331, 361, 362, 363, 364];
const COND_HELP_HZ: [u32; 21] = [322, 316, 315, 317, 318, 320, 325, 319, 321, 326, 0, 0, 333, 334, 314, 314, 335, 312, 313, 324, 311];

pub fn cond_icons(d: &crossbell_tool::kai::compare::Data) -> HashMap<usize, u32> {
	let idx = |id: &str, pre: &str| -> Option<(usize, String)> {
		let (k, f) = id.strip_prefix(pre)?.split_once('.')?;
		Some((k.parse().ok()?, f.to_string()))
	};
	let fname = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
	let mut rows = 0usize;
	for p in &d.pairs {
		if fname(&p.file).starts_with("t_condition_info.") {
			if let Some((k, _)) = idx(&p.id, "ConditionInfoTableData#") {
				rows = rows.max(k + 1);
			}
		}
	}
	let table: &[u32] = match rows {
		68 => &COND_HZ,
		54 => &COND_D1,
		59 => &COND_D2,
		_ => &[],
	};
	let mut m = HashMap::new();
	for (i, p) in d.pairs.iter().enumerate() {
		let f = fname(&p.file);
		let icon = if f.starts_with("t_condition_info.") {
			idx(&p.id, "ConditionInfoTableData#").filter(|(_, f)| f == "state_name").and_then(|(k, _)| table.get(k).copied())
		} else if f.starts_with("t_itemhelp.") && rows == 68 {
			idx(&p.id, "ConditionHelpData#").filter(|(_, f)| f == "text1" || f == "text2").and_then(|(k, _)| COND_HELP_HZ.get(k).copied())
		} else {
			None
		};
		let icon = icon.or_else(|| p.en.as_ref().filter(|s| s.text.trim() == "Stun").map(|_| 323));
		if let Some(c) = icon.filter(|&c| c > 0) {
			m.insert(i, c);
		}
	}
	m
}

pub fn sibling_icons(d: &crossbell_tool::kai::compare::Data) -> HashMap<usize, u32> {
	let mut icon_of: HashMap<(String, String), u32> = HashMap::new();
	for p in &d.pairs {
		let Some(t) = p.en.as_ref().map(|s| s.text.trim().to_string()) else { continue };
		if let Some(n) = t.strip_prefix("<I").and_then(|x| x.strip_suffix('>')).and_then(|x| x.parse::<u32>().ok()) {
			if let Some((rec, _)) = p.id.rsplit_once('.') {
				icon_of.insert((p.file.clone(), rec.to_string()), n);
			}
		}
	}
	let mut m = HashMap::new();
	for (i, p) in d.pairs.iter().enumerate() {
		if !p.id.starts_with("FreeDungeonReward#") {
			continue;
		}
		let Some((rec, field)) = p.id.rsplit_once('.') else { continue };
		if field == "text" {
			if let Some(&n) = icon_of.get(&(p.file.clone(), rec.to_string())) {
				m.insert(i, n);
			}
		}
	}
	m
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
		let clean = crossbell_tool::kai::compare::clean(&marked, true);
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


#[cfg(test)]
mod tests {
	use super::*;

	fn tbl_rows(d: &[u8], section: &str) -> usize {
		let u32at = |o: usize| d.get(o..o + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize);
		(0..u32at(4).unwrap_or(0)).find_map(|i| (d.get(8 + i * 80..8 + i * 80 + 64)?.split(|&c| c == 0).next()? == section.as_bytes()).then(|| u32at(8 + i * 80 + 76)).flatten()).unwrap_or(0)
	}

	#[test]
	fn used_cells() {
		let Ok(dir) = std::env::var("USED_CELLS") else { return };
		let dir = std::path::Path::new(&dir);
		let mut cells = std::collections::BTreeSet::new();
		if let Ok(b) = std::fs::read(dir.join("t_item.tbl")) {
			cells.extend(parse_kai_t_item(&b).into_values());
			cells.extend(parse_kai_item_tail(&b).into_values().flatten());
		}
		let rows = std::fs::read(dir.join("t_condition_info.tbl")).ok().map(|b| tbl_rows(&b, "ConditionInfoTableData")).unwrap_or(0);
		let table: &[u32] = match rows {
			68 => &COND_HZ,
			54 => &COND_D1,
			59 => &COND_D2,
			_ => &[],
		};
		cells.extend(table.iter().copied().filter(|&c| c > 0));
		if rows == 68 {
			cells.extend(COND_HELP_HZ.iter().copied().filter(|&c| c > 0));
		}
		cells.insert(323);
		if dir.join("t_dlc.tbl").is_file() {
			cells.insert(6);
		}
		for n in 800..1000 {
			cells.extend(ui_icon("t_text.tbl", &format!("TextTableData#{n}.value")));
		}
		if dir.join("t_free_dungeon.tbl").is_file() {
			for k in 0..20 {
				cells.extend(ui_icon("t_free_dungeon.tbl", &format!("FreeDungeonMassEffectText#{k}.text")));
			}
		}
		if let Ok(b) = std::fs::read(dir.join("t_skill.tbl")) {
			cells.extend(parse_kai_t_skill(&b).into_values());
			cells.extend(parse_kai_skill_rank(&b).into_values().flatten());
		}
		if let Ok(b) = std::fs::read(dir.join("t_itemhelp.tbl")) {
			for (sec, at) in [("SkillRangeHelpData", 16), ("SkillEffectHelpData", 68), ("SkillEffectHelpData", 64), ("SkillItemStatusData", 32), ("AttrTypeHelpData", 16)] {
				cells.extend(parse_kai_section_icon(&b, sec, at).into_values());
			}
		}
		for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
			let b = std::fs::read(e.path()).unwrap_or_default();
			let t = String::from_utf8_lossy(&b);
			for (i, _) in t.match_indices("<I") {
				let n: String = t[i + 2..].chars().take_while(|c| c.is_ascii_digit()).collect();
				if let Ok(n) = n.parse::<u32>() {
					cells.insert(n);
				}
			}
		}
		let s: Vec<String> = cells.into_iter().filter(|&c| c < ATLAS_MAX).map(|c| c.to_string()).collect();
		println!("CELLS {}", s.join(" "));
	}
}
