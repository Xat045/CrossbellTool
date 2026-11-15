#![cfg_attr(windows, windows_subsystem = "windows")]
#[allow(unused_imports)]
use crossbell_tool::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crossbell_tool::lang::tr;

use crossbell_tool::crossbell::{dt, itp, jobs, scena, scena_json};
mod gui_common;
mod crossbell_gui;
mod crossbell_pick;

use std::path::PathBuf;
use scena::Title;

struct Console;
impl jobs::Sink for Console {
	fn log(&self, s: &str) { println!("{s}") }
	fn progress(&self, _d: usize, _t: usize) {}
}

const USAGE: &str = "\
CrossbellTool: decompiles and builds the files of Trails from Zero / Trails to Azure

Without parameters, the program window opens.

Command line:
  CrossbellTool decompile --game zero|azure --data <game folder> --proj <project folder>
                       [--types scena,tables,textures,other] [--mask c0100,t_town] [--overwrite] [--dds]
  CrossbellTool compile   --proj <project folder> --out <output folder>
                       [--types ...] [--mask ...] [--all] [--no-check]
  (--dds keeps textures as .dds without re-encoding instead of .png; the build understands both)
  CrossbellTool file <file>      one file: .bin/._dt/.dt2/.itp -> .json/.xlsx/.png and back;
                                .itc -> folder <name>_frames with frames, folder -> <name>_new.itc
                                (.bin needs --game; a folder needs <name>.itc next to it)";

fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();
	if args.is_empty() && (cfg!(windows) || std::env::var("KISEKI_GUI").is_ok()) {
		crossbell_gui::run();
		return;
	}
	#[cfg(windows)]
	{
		unsafe {
			windows_sys::Win32::System::Console::AttachConsole(windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS);
		}
	}
	std::process::exit(cli(args));
}

fn cli(args: Vec<String>) -> i32 {
	let Some(cmd) = args.first().cloned() else {
		println!("{USAGE}");
		return 0;
	};
	let mut o = jobs::Options {
		title: Title::Zero,
		game_dir: PathBuf::new(),
		proj_dir: PathBuf::new(),
		out_dir: PathBuf::new(),
		scena: true,
		tables: true,
		textures: true,
		other: false,
		keep_failed: true,
		tex_dds: false,
		check: true,
		mask: String::new(),
		only_changed: true,
		overwrite_edited: false,
		include: None,
		build_only: None,
	};
	let mut file = None;
	let mut game_given = false;
	let mut i = 1;
	while i < args.len() {
		let a = args[i].as_str();
		let mut val = || { i += 1; args.get(i).cloned().unwrap_or_default() };
		match a {
			"--game" => {
				game_given = true;
				o.title = match val().to_lowercase().as_str() {
					"zero" => Title::Zero,
					"azure" | "ao" => Title::Azure,
					g => { leprintln!("неизвестная игра: {g}", "unknown game: {g}"); return 2 }
				}
			}
			"--data" => o.game_dir = val().into(),
			"--proj" => o.proj_dir = val().into(),
			"--out" => o.out_dir = val().into(),
			"--mask" => o.mask = val(),
			"--types" => {
				let t = val().to_lowercase();
				o.scena = t.contains("scena");
				o.tables = t.contains("table");
				o.textures = t.contains("texture");
				o.other = t.contains("other");
			}
			"--all" => o.only_changed = false,
			"--overwrite" => o.overwrite_edited = true,
			"--dds" => o.tex_dds = true,
			"--no-check" => o.check = false,
			f if !f.starts_with("--") => file = Some(PathBuf::from(f)),
			_ => { leprintln!("неизвестный параметр: {a}\n\n{USAGE}", "unknown parameter: {a}\n\n{USAGE}"); return 2 }
		}
		i += 1;
	}
	if !game_given
		&& let Some(g) = jobs::project_game(&o.proj_dir) {
			o.title = g;
		}
	let s = match cmd.as_str() {
		"decompile" => jobs::decompile(&o, &Console),
		"compile" => jobs::compile(&o, &Console),
		"file" => return single_file(&o, file),
		_ => { println!("{USAGE}"); return 2 }
	};
	if s.failed > 0 { 1 } else { 0 }
}

fn sprite_frames(f: &std::path::Path, data: &[u8]) -> i32 {
	let dir = f.with_file_name(format!("{}_frames", f.file_stem().unwrap_or_default().to_string_lossy()));
	let r = itp::itc_decode(data).and_then(|frames| {
		std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
		frames.iter().enumerate().try_for_each(|(i, png)| std::fs::write(dir.join(format!("{i:03}.png")), png).map_err(|e| e.to_string()))
	});
	match r {
		Ok(()) => { println!("{}", dir.display()); 0 }
		Err(e) => { leprintln!("ошибка: {e}", "error: {e}"); 1 }
	}
}

fn sprite_back(dir: &std::path::Path) -> i32 {
	let name = dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
	let Some(base) = name.strip_suffix("_frames") else { leprintln!("ошибка: нужна папка <имя>_frames", "error: a <name>_frames folder is needed"); return 2 };
	let orig_path = dir.with_file_name(format!("{base}.itc"));
	let r = (|| -> Result<PathBuf, String> {
		let orig = std::fs::read(&orig_path).map_err(|e| lformat!("нужен оригинал {} рядом: {e}", "the original {} is needed next to it: {e}", orig_path.display()))?;
		let mut edited = Vec::new();
		for (i, png) in itp::itc_decode(&orig)?.into_iter().enumerate() {
			let cur = std::fs::read(dir.join(format!("{i:03}.png"))).map_err(|e| e.to_string())?;
			if cur != png {
				edited.push((i, cur));
			}
		}
		let out = dir.with_file_name(format!("{base}_new.itc"));
		std::fs::write(&out, itp::itc_replace(&orig, &edited)?).map_err(|e| e.to_string())?;
		Ok(out)
	})();
	match r {
		Ok(p) => { println!("{}", p.display()); 0 }
		Err(e) => { leprintln!("ошибка: {e}", "error: {e}"); 1 }
	}
}

fn single_file(o: &jobs::Options, file: Option<PathBuf>) -> i32 {
	let Some(f) = file else { leprintln!("не указан файл", "no file given"); return 2 };
	let name = f.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
	let stem = name.split('.').next().unwrap_or("").to_string();
	if f.is_dir() {
		return sprite_back(&f);
	}
	let data = match std::fs::read(&f) { Ok(d) => d, Err(e) => { eprintln!("{e}"); return 1 } };
	let res: Result<(PathBuf, Vec<u8>), String> = if name.ends_with(".bin") {
		scena::decompile_json(o.title, &data).map(|(t, w)| {
			if let Some(w) = w { leprintln!("предупреждение: {w}", "warning: {w}") }
			(f.with_extension("json"), t.into_bytes())
		})
	} else if name.ends_with(".itc") {
		return sprite_frames(&f, &data);
	} else if name.ends_with("._dt") || name.ends_with(".dt2") {
		dt::decompile_xlsx(&data, &stem, &name).map(|(b, _)| (f.with_file_name(format!("{stem}.xlsx")), b))
	} else if name.ends_with(".itp") {
		itp::decompile(&data).map(|d| (f.with_extension("png"), d.png))
	} else if name.ends_with(".png") {
		itp::compile(&data, 0, None).map(|(d, _)| (f.with_extension("itp"), d))
	} else if name.ends_with(".xlsx") {
		let ext = if stem.starts_with("monsnote") { "dt2" } else { "_dt" };
		dt::compile_xlsx(&data).map(|d| (f.with_file_name(format!("{stem}.{ext}")), d))
	} else if name.ends_with(".json") || name.ends_with(".txt") {
		let src = String::from_utf8_lossy(&data).into_owned();
		if scena_json::looks_like(&src) || scena::looks_like_scena_text(&src) {
			scena::compile_any(&src, &name).map(|(d, _)| (f.with_extension("bin"), d))
		} else if dt::looks_like_table_text(&src) {
			dt::compile(&src).map(|d| (f.with_file_name(format!("{stem}._dt")), d))
		} else {
			Err(tr("не похоже на файл, созданный этой программой", "does not look like a file created by this program").into())
		}
	} else {
		Err(tr("неизвестный тип файла", "unknown file type").into())
	};
	match res {
		Ok((p, d)) => match std::fs::write(&p, d) {
			Ok(()) => { println!("{}", p.display()); 0 }
			Err(e) => { eprintln!("{e}"); 1 }
		},
		Err(e) => { leprintln!("ошибка: {e}", "error: {e}"); 1 }
	}
}
