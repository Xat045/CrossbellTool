use std::sync::atomic::{AtomicBool, Ordering};

pub static UI_EN: AtomicBool = AtomicBool::new(true);

pub fn ui_en() -> bool {
	UI_EN.load(Ordering::Relaxed)
}

pub fn tr(ru: &'static str, en: &'static str) -> &'static str {
	if ui_en() { en } else { ru }
}

#[macro_export]
macro_rules! lformat {
	($ru:literal, $en:literal $(, $($a:tt)*)?) => {
		if $crate::lang::ui_en() { format!($en $(, $($a)*)?) } else { format!($ru $(, $($a)*)?) }
	};
}

#[macro_export]
macro_rules! lprintln {
	($ru:literal, $en:literal $(, $($a:tt)*)?) => {
		if $crate::lang::ui_en() { println!($en $(, $($a)*)?) } else { println!($ru $(, $($a)*)?) }
	};
}

#[macro_export]
macro_rules! leprintln {
	($ru:literal, $en:literal $(, $($a:tt)*)?) => {
		if $crate::lang::ui_en() { eprintln!($en $(, $($a)*)?) } else { eprintln!($ru $(, $($a)*)?) }
	};
}
