#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use serde_json::{json, Map, Value};

pub const MAGIC: &str = "scena";

fn split_indent(line: &str) -> (usize, &str) {
	let n = line.bytes().take_while(|&b| b == b'\t').count();
	(n, &line[n..])
}

fn is_text_open(content: &str) -> bool {
	(content == "{" || content.ends_with(" {")) && !content.ends_with("\\{")
}

fn parse_block(lines: &[&str], i: &mut usize, indent: usize) -> Result<Vec<Value>, String> {
	let mut out = Vec::new();
	while *i < lines.len() {
		let raw = lines[*i];
		if raw.trim().is_empty() {
			*i += 1;
			continue;
		}
		let (ind, content) = split_indent(raw);
		if ind < indent {
			return Ok(out);
		}
		if ind > indent {
			return Err(lformat!("строка {}: неожиданный отступ", "line {}: unexpected indent", *i + 1));
		}
		if is_text_open(content) {
			let cmd = content[..content.len() - 1].trim_end().to_string();
			let close = format!("{}}}", "\t".repeat(ind));
			let next = format!("{}}} {{", "\t".repeat(ind));
			let inner = "\t".repeat(ind + 1);
			let mut pages: Vec<Value> = Vec::new();
			let mut cur: Vec<String> = Vec::new();
			*i += 1;
			loop {
				let Some(&l) = lines.get(*i) else {
					return Err(lformat!("строка {}: текст не закрыт '}}'", "line {}: text is not closed with '}}'", *i));
				};
				*i += 1;
				if l == close || l == next {
					pages.push(Value::String(cur.join("\n")));
					cur.clear();
					if l == close { break }
					continue;
				}
				cur.push(l.strip_prefix(&inner).unwrap_or(l.trim_start_matches('\t')).to_string());
			}
			out.push(json!({ "line": cmd, "text": pages }));
			continue;
		}
		let mut j = *i + 1;
		while j < lines.len() && lines[j].trim().is_empty() {
			j += 1;
		}
		if j < lines.len() && split_indent(lines[j]).0 > ind {
			*i = j;
			let body = parse_block(lines, i, ind + 1)?;
			out.push(json!({ "line": content, "body": body }));
		} else {
			out.push(Value::String(content.to_string()));
			*i += 1;
		}
	}
	Ok(out)
}

pub fn to_json(clm: &str) -> Result<String, String> {
	let clm = clm.replace("\r\n", "\n");
	let lines: Vec<&str> = clm.split('\n').collect();
	let mut i = 0;
	let script = parse_block(&lines, &mut i, 0)?;
	if i < lines.len() {
		return Err(lformat!("строка {}: не удалось разобрать структуру", "line {}: could not parse the structure", i + 1));
	}
	let mut root = Map::new();
	root.insert("kiseki".into(), Value::String(MAGIC.into()));
	root.insert(
		tr("_справка", "_help").into(),
		Value::String(tr("Полный скрипт сцены. \"text\" — окна диалога (\\n = новая строка в игре, {wait} — ждать нажатия). \"body\" — вложенные команды. Порядок элементов не меняйте.", "The full script of the scene. \"text\" holds dialogue windows (\\n is a new line in the game, {wait} waits for a key press). \"body\" holds nested commands. Do not change the order of the elements.").into()),
	);
	root.insert("script".into(), Value::Array(script));
	serde_json::to_string_pretty(&Value::Object(root)).map_err(|e| e.to_string())
}

fn emit(v: &Value, indent: usize, out: &mut String) -> Result<(), String> {
	let tabs = "\t".repeat(indent);
	match v {
		Value::String(s) => {
			out.push_str(&tabs);
			out.push_str(s);
			out.push('\n');
		}
		Value::Object(m) => {
			let line = m.get("line").and_then(|l| l.as_str()).ok_or(tr("объект без поля \"line\"", "object without a \"line\" field"))?;
			if let Some(text) = m.get("text") {
				let pages = text.as_array().ok_or(tr("\"text\" должен быть списком строк", "\"text\" must be a list of strings"))?;
				out.push_str(&tabs);
				out.push_str(line);
				out.push_str(if line.is_empty() { "{" } else { " {" });
				out.push('\n');
				for (n, p) in pages.iter().enumerate() {
					let p = p.as_str().ok_or_else(|| lformat!("в \"{line}\": страница текста должна быть строкой", "in \"{line}\": a text page must be a string"))?;
					if n > 0 {
						out.push_str(&tabs);
						out.push_str("} {\n");
					}
					if !p.is_empty() {
						for l in p.split('\n') {
							let l = l.strip_suffix('\r').unwrap_or(l);
							if !l.is_empty() {
								out.push_str(&tabs);
								out.push('\t');
								out.push_str(l);
							}
							out.push('\n');
						}
					}
				}
				out.push_str(&tabs);
				out.push_str("}\n");
			} else {
				out.push_str(&tabs);
				out.push_str(line);
				out.push('\n');
				if let Some(body) = m.get("body") {
					for b in body.as_array().ok_or(tr("\"body\" должен быть списком", "\"body\" must be a list"))? {
						emit(b, indent + 1, out)?;
					}
				}
			}
		}
		_ => return Err(tr("в \"script\" допускаются только строки и объекты", "\"script\" may only contain strings and objects").into()),
	}
	Ok(())
}

pub fn from_json(src: &str) -> Result<String, String> {
	let src = src.strip_prefix('\u{FEFF}').unwrap_or(src);
	let v: Value = serde_json::from_str(src).map_err(|e| lformat!("ошибка JSON (строка {}, столбец {}): {}", "JSON error (line {}, column {}): {}", e.line(), e.column(), json_err_ru(&e)))?;
	let script = v.get("script").and_then(|s| s.as_array()).ok_or(tr("нет списка \"script\"", "no \"script\" list"))?;
	let mut out = String::new();
	for (n, item) in script.iter().enumerate() {
		if n > 0 && matches!(item, Value::Object(_)) {
			out.push('\n');
		}
		emit(item, 0, &mut out)?;
	}
	Ok(out)
}

pub fn looks_like(src: &str) -> bool {
	let s = src.trim_start_matches('\u{FEFF}').trim_start();
	s.starts_with('{') && s.contains("\"kiseki\"") && s.contains("\"scena\"")
}

fn json_err_ru(e: &serde_json::Error) -> String {
	let m = e.to_string();
	let m = m.split(" at line").next().unwrap_or(&m).to_string();
	if m.contains("expected `,` or `}`") || m.contains("expected `,` or `]`") {
		tr("пропущена запятая или скобка (часто — неэкранированная кавычка \" внутри текста, пишите \\\")", "a comma or bracket is missing (often an unescaped quote \" inside the text, write \\\")").into()
	} else if m.contains("control character") {
		tr("в строке настоящий перевод строки или табуляция — используйте \\n", "the string has a real line break or tab: use \\n").into()
	} else if m.contains("trailing comma") {
		tr("лишняя запятая перед ] или }", "an extra comma before ] or }").into()
	} else if m.contains("invalid escape") {
		tr("неверная escape-последовательность: обратный слэш пишется как \\\\", "invalid escape sequence: a backslash is written as \\\\").into()
	} else {
		m
	}
}
