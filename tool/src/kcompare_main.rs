#![cfg_attr(windows, windows_subsystem = "windows")]
#[allow(unused_imports)]
use crossbell_tool::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crossbell_tool::lang::tr;

mod gui_common;
mod compare_gui;
mod kai_buttons;

fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();
	if args.len() == 3 && args[0] == "--pack" {
		match kai_buttons::pack::build(args[1].as_ref(), args[2].as_ref()) {
			Ok(n) if n > 0 => std::process::exit(0),
			_ => std::process::exit(1),
		}
	}
	compare_gui::run_ed7_main();
}
