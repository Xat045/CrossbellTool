#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use themelios::scena::ed7::Scena;
use themelios::types::Game;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Title {
	Zero,
	Azure,
}

impl Title {
	pub fn game(self) -> Game {
		match self {
			Title::Zero => Game::ZeroKai,
			Title::Azure => Game::AoKai,
		}
	}
	pub fn label(self) -> &'static str {
		match self {
			Title::Zero => "Trails from Zero",
			Title::Azure => "Trails to Azure",
		}
	}
}

pub fn looks_like_scena_text(src: &str) -> bool {
	src.trim_start_matches('\u{FEFF}').trim_start().starts_with("calmare")
}

fn read(title: Title, data: &[u8]) -> Result<Scena, String> {
	Scena::read(title.game(), data).map_err(|e| lformat!("не удалось разобрать сценарий: {e}", "could not parse the script: {e}"))
}

pub fn decompile(title: Title, data: &[u8]) -> Result<(String, Option<String>), String> {
	decompile_inner(title, data).map(|(text, warn, _)| (text, warn))
}

fn decompile_inner(title: Title, data: &[u8]) -> Result<(String, Option<String>, Vec<u8>), String> {
	let game = title.game();
	let scena = read(title, data)?;
	let text = calmare::to_string(game, &calmare::Content::ED7Scena(scena), None);

	let (bin, _) = compile_inner(&text, tr("<проверка>", "<check>"))?;
	let mut warn = None;
	if bin != data {
		match read(title, &bin) {
			Ok(s2) => {
				let text2 = calmare::to_string(game, &calmare::Content::ED7Scena(s2), None);
				if text2 != text {
					warn = Some(tr("после пересборки скрипт отличается — проверьте этот файл в игре", "after rebuilding the script differs: check this file in the game").to_string());
				}
			}
			Err(e) => return Err(lformat!("самопроверка не прошла: {e}", "self-check failed: {e}")),
		}
	}
	Ok((text, warn, bin))
}

pub fn decompile_json(title: Title, data: &[u8]) -> Result<(String, Option<String>), String> {
	let (clm, warn, a) = decompile_inner(title, data)?;
	let json = crate::scena_json::to_json(&clm)?;
	let clm2 = crate::scena_json::from_json(&json).map_err(|e| lformat!("самопроверка JSON: {e}", "JSON self-check: {e}"))?;
	let (b, _) = compile_inner(&clm2, tr("<проверка JSON>", "<JSON check>")).map_err(|e| lformat!("самопроверка JSON: {e}", "JSON self-check: {e}"))?;
	if a != b {
		return Err(tr("внутренняя ошибка: JSON собирается иначе, чем исходный скрипт", "internal error: the JSON builds differently from the source script").into());
	}
	Ok((json, warn))
}

pub fn compile_any(src: &str, file_name: &str) -> Result<(Vec<u8>, Vec<String>), String> {
	if crate::scena_json::looks_like(src) {
		let clm = crate::scena_json::from_json(src)?;
		match compile_inner(&clm, file_name) {
			Ok(r) => Ok(r),
			Err(_) => Err(json_errors(src, &clm)),
		}
	} else {
		compile_inner(src, file_name)
	}
}

pub fn compile(src: &str, file_name: &str) -> Result<(Vec<u8>, Vec<String>), String> {
	compile_inner(src, file_name)
}

fn compile_inner(src: &str, file_name: &str) -> Result<(Vec<u8>, Vec<String>), String> {
	let src = src.strip_prefix('\u{FEFF}').unwrap_or(src);
	let src = crate::text::normalize(src);
	let src = if src.contains('\r') { src.replace("\r\n", "\n").replace('\r', "\n") } else { src.into_owned() };
	let (val, diags) = calmare::parse(&src, None);
	let rendered = render_diags(file_name, &src, &diags);
	let fatal = diags.iter().any(|d| d.is_fatal());
	match val {
		Some((game, calmare::Content::ED7Scena(s))) if !fatal => {
			let data = Scena::write(game, &s).map_err(|e| lformat!("ошибка записи: {e}", "write error: {e}"))?;
			Ok((data, rendered))
		}
		Some((_, calmare::Content::ED6Scena(_))) if !fatal => {
			Err(tr("это сценарий ED6 (Trails in the Sky), а не Zero/Azure", "this is an ED6 script (Trails in the Sky), not Zero/Azure").into())
		}
		_ => {
			let mut msg = String::from(tr("ошибки в скрипте:\n", "errors in the script:\n"));
			for r in &rendered {
				msg.push_str(r);
			}
			let all = rendered.concat();
			if all.contains("Cannot encode") {
				msg.push_str(tr("Подсказка: в тексте есть символ, которого нет в кодировке игры (Shift-JIS). Частые случаи: «ёлочки» и „лапки“ — замените на обычные кавычки \", длинное тире — на ―.\n", "Hint: the text has a character that is missing from the game's encoding (Shift-JIS). Common cases: «guillemets» and „low quotes“: replace them with plain quotes \", and a long dash with ―.\n"));
			}
			Err(msg.trim_end().to_string())
		}
	}
}

fn render_diags(file_name: &str, src: &str, diags: &[calmare::parse::Diag]) -> Vec<String> {
	use codespan_reporting::diagnostic::{Diagnostic, Label};
	use codespan_reporting::files::SimpleFiles;
	use codespan_reporting::term::termcolor::NoColor;
	use calmare::parse::diag::Level;

	let config = codespan_reporting::term::Config::default();
	let mut files = SimpleFiles::new();
	let id = files.add(file_name, src);
	let mut diags = diags.to_owned();
	diags.sort_by_key(|a| (a.text.0.start, a.text.0.end));
	let mut out = Vec::new();
	for d in diags {
		let mut labels = vec![Label::primary(id, d.text.0.as_range()).with_message(&d.text.1)];
		for n in &d.notes {
			labels.push(Label::secondary(id, n.0.as_range()).with_message(&n.1));
		}
		let diag = match d.level {
			Level::Error => Diagnostic::error().with_message(tr("ошибка", "error")),
			Level::Warning => Diagnostic::warning().with_message(tr("предупреждение", "warning")),
			Level::Info => Diagnostic::help(),
		}
		.with_labels(labels);
		let mut buf = NoColor::new(Vec::new());
		let _ = codespan_reporting::term::emit(&mut buf, &config, &files, &diag);
		out.push(String::from_utf8_lossy(&buf.into_inner()).into_owned());
	}
	out
}

fn json_errors(json_src: &str, clm: &str) -> String {
	let clm = crate::text::normalize(clm).into_owned();
	let (_, diags) = calmare::parse(&clm, None);
	let json_norm = crate::text::normalize(json_src).into_owned();
	let json_lines: Vec<&str> = json_norm.split('\n').collect();
	let mut out = String::from(tr("ошибки в скрипте:\n", "errors in the script:\n"));
	let mut any = false;
	for d in diags.iter().filter(|d| d.is_fatal()) {
		any = true;
		let range = d.text.0.as_range();
		let start = range.start.min(clm.len());
		let end = range.end.clamp(start, clm.len());
		let line_start = clm[..start].rfind('\n').map(|p| p + 1).unwrap_or(0);
		let line_end = clm[start..].find('\n').map(|p| start + p).unwrap_or(clm.len());
		let line = clm[line_start..line_end].trim_start_matches('\t');
		let mut cands: Vec<String> = Vec::new();
		if let Some(q) = d.text.1.split_once("encode \"").map(|x| x.1) {
			if let Some(q) = q.rsplit_once("\" as") { cands.push(q.0.to_string()) }
		}
		for l in clm[line_start..end.max(line_end)].split('\n').skip(1) {
			let l = l.trim_start_matches('\t');
			if l.len() > 3 && l != "}" && l != "} {" { cands.push(l.to_string()) }
		}
		cands.push(line.to_string());
		let found = cands.iter().find_map(|c| {
			let esc = serde_json::to_string(c).unwrap_or_default();
			let esc = esc.trim_matches('"');
			let key: String = esc.chars().take(50).collect();
			if key.chars().count() < 3 { return None }
			json_lines.iter().position(|l| l.contains(key.as_str()))
		});
		match found {
			Some(n) => out.push_str(&lformat!("строка {} (JSON): {}\n    {}\n", "line {} (JSON): {}\n    {}\n", n + 1, d.text.1, json_lines[n].trim())),
			None => out.push_str(&format!("{}\n    {}\n", d.text.1, line)),
		}
	}
	if !any {
		out.push_str(tr("неизвестная ошибка сборки\n", "unknown build error\n"));
	}
	if out.contains("Cannot encode") {
		out.push_str(tr("Подсказка: в тексте есть символ, которого нет в кодировке игры (Shift-JIS). Частые случаи: «ёлочки» и „лапки“ — замените на обычные кавычки (в JSON пишутся как \\\"), короткое тире – на -.\n", "Hint: the text has a character that is missing from the game's encoding (Shift-JIS). Common cases: «guillemets» and „low quotes“: replace them with plain quotes (written as \\\" in JSON), and a short dash – with -.\n"));
	}
	out.trim_end().to_string()
}

pub fn quick_json(title: Title, data: &[u8]) -> Result<String, String> {
	let scena = read(title, data)?;
	let text = calmare::to_string(title.game(), &calmare::Content::ED7Scena(scena), None);
	crate::scena_json::to_json(&text)
}

impl Title {
	pub fn key(self) -> &'static str {
		match self {
			Title::Zero => "zero",
			Title::Azure => "azure",
		}
	}
	pub fn other(self) -> Title {
		match self {
			Title::Zero => Title::Azure,
			Title::Azure => Title::Zero,
		}
	}
}

pub fn detect_title(dir: &std::path::Path) -> Option<Title> {
	let mut cur = Some(dir);
	let mut depth = 0;
	while let Some(d) = cur {
		if d.join("zero.exe").is_file() {
			return Some(Title::Zero);
		}
		if d.join("ao.exe").is_file() {
			return Some(Title::Azure);
		}
		let n = d.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
		if n.contains("azure") {
			return Some(Title::Azure);
		}
		if n.contains("zero") {
			return Some(Title::Zero);
		}
		depth += 1;
		if depth > 4 {
			break;
		}
		cur = d.parent();
	}
	None
}
