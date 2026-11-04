#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Game {
	Daybreak,
	Daybreak2,
	Horizon,
}

static CURRENT: AtomicU8 = AtomicU8::new(2);

impl Game {
	pub fn name(self) -> &'static str {
		match self {
			Game::Daybreak => "Trails through Daybreak",
			Game::Daybreak2 => "Trails through Daybreak II",
			Game::Horizon => "Trails beyond the Horizon",
		}
	}
	pub fn schema_tag(self) -> &'static str {
		match self {
			Game::Daybreak => "Kuro1",
			Game::Daybreak2 => "Kuro2",
			Game::Horizon => "Kai",
		}
	}
	fn code(self) -> u8 {
		match self { Game::Daybreak => 0, Game::Daybreak2 => 1, Game::Horizon => 2 }
	}
	pub fn from_code(c: u8) -> Game {
		match c { 0 => Game::Daybreak, 1 => Game::Daybreak2, _ => Game::Horizon }
	}
	pub fn key(self) -> &'static str {
		match self { Game::Daybreak => "daybreak", Game::Daybreak2 => "daybreak2", Game::Horizon => "horizon" }
	}
	pub fn from_key(s: &str) -> Option<Game> {
		let s = s.trim().to_lowercase().replace([' ', '-', '_'], "");
		Some(match s.as_str() {
			"daybreak" | "daybreak1" | "kuro" | "kuro1" => Game::Daybreak,
			"daybreak2" | "daybreakii" | "kuro2" => Game::Daybreak2,
			"horizon" | "kai" => Game::Horizon,
			_ => return None,
		})
	}
}

pub fn set(g: Game) {
	CURRENT.store(g.code(), Ordering::Relaxed)
}

pub fn current() -> Game {
	Game::from_code(CURRENT.load(Ordering::Relaxed))
}

pub fn detect(dir: &Path) -> Option<Game> {
	let mut d = Some(dir);
	for _ in 0..3 {
		let Some(p) = d else { break };
		if let Some(g) = detect_here(p) {
			return Some(g);
		}
		d = p.parent();
	}
	None
}

fn detect_here(p: &Path) -> Option<Game> {
	if let Ok(s) = std::fs::read_to_string(p.join(".kai_game.txt")) {
		if let Some(g) = Game::from_key(&s) {
			return Some(g);
		}
	}
	let has = |n: &str| p.join(n).exists();
	if has("kuro2.exe") {
		return Some(Game::Daybreak2);
	}
	if has("kuro.exe") {
		return Some(Game::Daybreak);
	}
	if has("kai.exe") || has("script_eng.p3a") {
		return Some(Game::Horizon);
	}
	if find_dir(p, "script_eng", 4) {
		return Some(Game::Horizon);
	}
	let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
	let norm: String = name.chars().map(|c| if c == '_' || c == '-' || c == '.' { ' ' } else { c }).collect();
	let second = |key: &str| {
		norm.match_indices(key).any(|(i, m)| {
			let rest = &norm[i + m.len()..];
			let tail = rest.trim_start();
			(tail.starts_with("ii") && !tail[2..].starts_with(|c: char| c.is_alphanumeric()))
				|| (tail.starts_with('2') && !tail[1..].starts_with(|c: char| c.is_ascii_digit()))
		})
	};
	if second("daybreak") || second("kuro no kiseki") || second("kuro") {
		return Some(Game::Daybreak2);
	}
	let third = norm.match_indices("daybreak").any(|(i, m)| {
		let tail = norm[i + m.len()..].trim_start();
		(tail.starts_with("iii") && !tail[3..].starts_with(|c: char| c.is_alphanumeric())) || (tail.starts_with('3') && !tail[1..].starts_with(|c: char| c.is_ascii_digit()))
	});
	if third {
		return Some(Game::Horizon);
	}
	if name.contains("daybreak") {
		return Some(Game::Daybreak);
	}
	if name.contains("horizon") {
		return Some(Game::Horizon);
	}
	None
}

fn find_dir(p: &Path, name: &str, depth: usize) -> bool {
	let Ok(rd) = std::fs::read_dir(p) else { return false };
	for e in rd.flatten() {
		if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
			continue;
		}
		let n = e.file_name().to_string_lossy().to_lowercase();
		if n == name {
			return true;
		}
		if depth > 1 && !n.starts_with('_') && !n.starts_with('.') && find_dir(&e.path(), name, depth - 1) {
			return true;
		}
	}
	false
}

pub fn remember(proj: &Path, g: Game) {
	let _ = std::fs::write(proj.join(".kai_game.txt"), g.key());
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn detect_by_folders_and_names() {
		let root = std::env::temp_dir().join(format!("kai_game_test_{}", std::process::id()));
		let mk = |rel: &str| {
			let p = root.join(rel);
			std::fs::create_dir_all(&p).unwrap();
			p
		};
		mk("Daybreak-3/RU/mods/RU_MOD/script_eng/scena");
		assert_eq!(detect(&root.join("Daybreak-3/RU")), Some(Game::Horizon));
		assert_eq!(detect(&mk("Daybreak 3")), Some(Game::Horizon));
		assert_eq!(detect(&mk("Daybreak-2")), Some(Game::Daybreak2));
		mk("Daybreak/EN/script_en/script_en/scena");
		assert_eq!(detect(&root.join("Daybreak/EN")), Some(Game::Daybreak));
		let _ = std::fs::remove_dir_all(&root);
	}
}
