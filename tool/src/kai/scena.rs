#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use serde_json::{Map, Value};

pub const MAGIC: &str = "kai-scena";

fn to_ing(data: &[u8]) -> Result<String, String> {
	if !data.starts_with(b"#scp") {
		return Err(tr("это не скрипт #scp", "this is not a #scp script").into());
	}
	let opts = ingert::scena::DecompileOptions {
		mode: ingert::scena::DecompileMode::Tree,
		called: true,
		roundtrip: true,
	};
	let scp = std::panic::catch_unwind(|| ingert::scp::read(data))
		.map_err(|_| tr("внутренняя ошибка Ingert при чтении", "internal Ingert error while reading").to_string())?
		.map_err(|e| lformat!("не удалось прочитать скрипт: {e}", "could not read the script: {e}"))?;
	let text = std::panic::catch_unwind(move || {
		let mut scena = ingert::scena::from_scp(scp);
		ingert::scena::decompile(&mut scena, &opts);
		ingert_syntax::print::print(&scena)
	})
	.map_err(|_| tr("внутренняя ошибка Ingert при декомпиляции", "internal Ingert error while decompiling").to_string())?;
	Ok(text)
}

fn from_ing(src: &str) -> Result<Vec<u8>, String> {
	use ingert_syntax::diag;
	let mut errors = diag::Errors::new();
	let tokens = ingert_syntax::lex::lex(src, &mut errors);
	let fatal = errors.max_severity() >= diag::Severity::Fatal;
	let scena = if fatal { None } else { Some(ingert_syntax::parse::parse(&tokens, &mut errors)) };
	if errors.max_severity() >= diag::Severity::Error || scena.is_none() {
		let mut msg = String::new();
		let mut errs: Vec<_> = errors.errors.iter().filter(|e| e.severity >= diag::Severity::Error).collect();
		errs.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
		for e in errs.iter().take(10) {
			let start = e.main.span.start.min(src.len());
			let line = src[..start].matches('\n').count() + 1;
			let text_line = src.lines().nth(line - 1).unwrap_or("").trim();
			msg.push_str(&lformat!("строка скрипта {line}: {}\n    {}\n", "script line {line}: {}\n    {}\n", e.main.desc, text_line.chars().take(200).collect::<String>()));
		}
		return Err(msg.trim_end().to_string());
	}
	let scena = scena.unwrap();
	let scp = std::panic::catch_unwind(move || ingert::scena::compile(scena))
		.map_err(|_| tr("внутренняя ошибка Ingert при сборке", "internal Ingert error while building").to_string())?
		.map_err(|e| lformat!("ошибка сборки: {e}", "build error: {e}"))?;
	ingert::scp::write(&scp).map_err(|e| lformat!("ошибка записи: {e}", "write error: {e}"))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum T {
	Str,
	Word,
	At,
	Open,
	Close,
	Comma,
	Other,
}

#[derive(Clone, Debug)]
struct Tok {
	t: T,
	a: usize,
	b: usize,
}

fn lex(line: &str) -> Vec<Tok> {
	let s = line.as_bytes();
	let mut out = Vec::new();
	let mut i = 0;
	while i < s.len() {
		let c = s[i];
		if c == b'"' || c == b'\'' {
			let a = i;
			i += 1;
			while i < s.len() && s[i] != c {
				if s[i] == b'\\' {
					i += 1;
				}
				i += 1;
			}
			i = (i + 1).min(s.len());
			out.push(Tok { t: T::Str, a, b: i });
		} else if c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || c >= 0x80 {
			let a = i;
			while i < s.len() && (s[i].is_ascii_alphanumeric() || s[i] == b'_' || s[i] == b'.' || s[i] >= 0x80) {
				i += 1;
			}
			out.push(Tok { t: T::Word, a, b: i });
		} else {
			let t = match c {
				b'@' => T::At,
				b'(' | b'[' | b'{' => T::Open,
				b')' | b']' | b'}' => T::Close,
				b',' => T::Comma,
				b' ' | b'\t' => {
					i += 1;
					continue;
				}
				_ => T::Other,
			};
			out.push(Tok { t, a: i, b: i + 1 });
			i += 1;
		}
	}
	out
}

fn unescape(lit: &str) -> String {
	let inner = &lit[1..lit.len().saturating_sub(1).max(1)];
	let mut out = String::new();
	let mut it = inner.chars().peekable();
	while let Some(c) = it.next() {
		if c != '\\' {
			out.push(c);
			continue;
		}
		match it.next() {
			Some('n') => out.push('\n'),
			Some('r') => out.push('\r'),
			Some('t') => out.push('\t'),
			Some('u') => {
				let mut hex = String::new();
				if it.peek() == Some(&'{') {
					it.next();
					while let Some(&h) = it.peek() {
						it.next();
						if h == '}' { break }
						hex.push(h);
					}
				}
				out.push(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32).unwrap_or('\u{FFFD}'));
			}
			Some(x) => out.push(x),
			None => out.push('\\'),
		}
	}
	out
}

fn escape(s: &str) -> String {
	let mut out = String::from("\"");
	for c in s.chars() {
		match c {
			'\\' => out.push_str("\\\\"),
			'\n' => out.push_str("\\n"),
			'\r' => out.push_str("\\r"),
			'\t' => out.push_str("\\t"),
			'"' => out.push_str("\\\""),
			c => out.push(c),
		}
	}
	out.push('"');
	out
}

fn tag_only(s: &str) -> bool {
	let mut rest = s;
	loop {
		if rest.is_empty() { return true }
		if !rest.starts_with('<') { return false }
		match rest.find('>') {
			Some(p) => rest = &rest[p + 1..],
			None => return false,
		}
	}
}

fn identifier_like(s: &str) -> bool {
	!s.contains(' ')
		&& s.is_ascii()
		&& (s.contains('_') || s.contains('.') || s.contains('/') || s.chars().all(|c| c.is_ascii_digit() || c == '-'))
}

fn has_letters(s: &str) -> bool {
	s.chars().any(|c| c.is_alphabetic())
}

#[derive(Clone, Debug)]
enum Piece {
	S(String),
	I(String),
}

#[derive(Debug)]
enum ArgKind {
	S { prefix: Option<String>, lit: String },
	I { prefix: Option<String>, num: String },
	Other,
}

fn classify(line: &str, toks: &[Tok]) -> ArgKind {
	let txt = |t: &Tok| &line[t.a..t.b];
	let (prefix, rest) = if toks.len() >= 3 && toks[0].t == T::Word && toks[1].t == T::At && txt(&toks[0]).bytes().all(|c| c.is_ascii_digit()) {
		(Some(txt(&toks[0]).to_string()), &toks[2..])
	} else {
		(None, toks)
	};
	match rest {
		[t] if t.t == T::Str && line.as_bytes()[t.a] == b'"' => ArgKind::S { prefix, lit: txt(t).to_string() },
		[t] if t.t == T::Word && txt(t).bytes().all(|c| c.is_ascii_digit()) => ArgKind::I { prefix, num: txt(t).to_string() },
		[m, t] if m.t == T::Other && txt(m) == "-" && t.t == T::Word && txt(t).bytes().all(|c| c.is_ascii_digit()) => {
			ArgKind::I { prefix, num: format!("-{}", txt(t)) }
		}
		_ => ArgKind::Other,
	}
}

const SINGLE_FUNCS: &[&str] = &[
	"menu_additem",
	"chr_set_display_name",
	"chr_set_shop_function",
	"ui_set_alignment_select_item",
	"ui_open_alignment_select",
];

struct Run {
	a: usize,
	b: usize,
	pieces: Vec<Piece>,
	prefixes: Vec<Option<String>>,
}

fn find_runs(line: &str) -> Vec<Run> {
	let toks = lex(line);
	let mut runs = Vec::new();
	let mut i = 0;
	while i < toks.len() {
		if !(toks[i].t == T::Open && &line[toks[i].a..toks[i].b] == "(") {
			i += 1;
			continue;
		}
		let callee = if i >= 1 && toks[i - 1].t == T::Word {
			line[toks[i - 1].a..toks[i - 1].b].to_string()
		} else if i >= 6 && toks[i - 1].t == T::Close && toks[i - 6].t == T::Word && &line[toks[i - 6].a..toks[i - 6].b] == "system" {
			line[toks[i - 6].a..toks[i - 1].b].replace(' ', "")
		} else {
			String::new()
		};
		let message = callee.starts_with("system[5,");
		let single = SINGLE_FUNCS.contains(&callee.as_str());
		let mut depth = 0i32;
		let mut j = i;
		let mut args: Vec<(usize, usize)> = Vec::new();
		let mut start = i + 1;
		while j < toks.len() {
			match toks[j].t {
				T::Open => depth += 1,
				T::Close => {
					depth -= 1;
					if depth == 0 {
						if j > start { args.push((start, j)) }
						break;
					}
				}
				T::Comma if depth == 1 => {
					args.push((start, j));
					start = j + 1;
				}
				_ => {}
			}
			j += 1;
		}
		if message || single {
			let kinds: Vec<ArgKind> = args.iter().map(|&(a, b)| classify(line, &toks[a..b])).collect();
			let is_text = |k: &ArgKind| match k {
				ArgKind::S { lit, .. } => {
					let u = unescape(lit);
					!tag_only(&u) && (message || !identifier_like(&u)) && has_letters(&u)
				}
				_ => false,
			};
			let mut k = 0;
			while k < kinds.len() {
				if !is_text(&kinds[k]) {
					k += 1;
					continue;
				}
				let first = k;
				let mut last = k;
				if message {
					while last + 1 < kinds.len() && (matches!(kinds[last + 1], ArgKind::I { .. }) || (matches!(&kinds[last + 1], ArgKind::S { lit, .. } if !unescape(lit).is_empty()) && matches!(kinds[last], ArgKind::I { .. }))) {
						last += 1;
					}
				}
				let mut pieces = Vec::new();
				let mut prefixes = Vec::new();
				for kk in &kinds[first..=last] {
					match kk {
						ArgKind::S { prefix, lit } => {
							pieces.push(Piece::S(unescape(lit)));
							prefixes.push(prefix.clone());
						}
						ArgKind::I { prefix, num } => {
							pieces.push(Piece::I(num.clone()));
							prefixes.push(prefix.clone());
						}
						ArgKind::Other => unreachable!(),
					}
				}
				runs.push(Run { a: toks[args[first].0].a, b: toks[args[last].1 - 1].b, pieces, prefixes });
				k = last + 1;
			}
		}
		i += 1;
	}
	runs.sort_by_key(|r| r.a);
	let mut out: Vec<Run> = Vec::new();
	for r in runs {
		if out.last().map(|l| r.a < l.b).unwrap_or(false) { continue }
		out.push(r);
	}
	out
}

fn pieces_to_text(p: &[Piece]) -> String {
	let mut s = String::new();
	for x in p {
		match x {
			Piece::I(n) if n == "10" => s.push('\n'),
			Piece::I(n) => s.push_str(&format!("{{{n}}}")),
			Piece::S(t) => {
				for c in t.chars() {
					match c {
						'{' => s.push_str("{{"),
						'\n' => s.push_str("{lf}"),
						'\r' => s.push_str("{cr}"),
						'\t' => s.push_str("{tab}"),
						c => s.push(c),
					}
				}
			}
		}
	}
	s
}

fn text_to_pieces(t: &str) -> Result<Vec<Piece>, String> {
	let t = t.replace("\r\n", "\n");
	let chars: Vec<char> = t.chars().collect();
	let mut out = Vec::new();
	let mut cur = String::new();
	let mut i = 0;
	let flush = |cur: &mut String, out: &mut Vec<Piece>| {
		if !cur.is_empty() {
			out.push(Piece::S(std::mem::take(cur)));
		}
	};
	while i < chars.len() {
		let c = chars[i];
		if c == '\n' {
			flush(&mut cur, &mut out);
			out.push(Piece::I("10".into()));
			i += 1;
			continue;
		}
		if c == '{' {
			if chars.get(i + 1) == Some(&'{') {
				cur.push('{');
				i += 2;
				continue;
			}
			let end = chars[i..].iter().position(|&x| x == '}').map(|p| i + p).ok_or(tr("одиночная '{' — для символа «{» пишите {{", "a single '{': for the '{' character write {{"))?;
			let code: String = chars[i + 1..end].iter().collect();
			match code.as_str() {
				"lf" => cur.push('\n'),
				"cr" => cur.push('\r'),
				"tab" => cur.push('\t'),
				n if { let d = n.strip_prefix('-').unwrap_or(n); !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()) } => {
					flush(&mut cur, &mut out);
					out.push(Piece::I(n.to_string()));
				}
				other => return Err(lformat!("неизвестный код {{{other}}}", "unknown code {{{other}}}")),
			}
			i = end + 1;
			continue;
		}
		cur.push(c);
		i += 1;
	}
	flush(&mut cur, &mut out);
	if out.is_empty() {
		out.push(Piece::S(String::new()));
	}
	Ok(out)
}

fn pieces_to_src(p: &[Piece], prefixes: Option<&[Option<String>]>) -> String {
	let use_pref = prefixes.filter(|x| x.len() == p.len());
	let mut parts = Vec::new();
	for (i, x) in p.iter().enumerate() {
		let pre = use_pref.and_then(|v| v[i].clone()).map(|v| format!("{v}@")).unwrap_or_default();
		match x {
			Piece::S(s) => parts.push(format!("{pre}{}", escape(s))),
			Piece::I(n) => parts.push(format!("{pre}{n}")),
		}
	}
	parts.join(", ")
}

fn ln_encode(p: &[Option<String>]) -> String {
	p.iter().map(|x| x.clone().unwrap_or_default()).collect::<Vec<_>>().join(",")
}
fn ln_decode(s: &str) -> Vec<Option<String>> {
	s.split(',').map(|x| if x.is_empty() { None } else { Some(x.to_string()) }).collect()
}

fn ing_to_json(ing: &str) -> (String, usize) {
	let mut script = Vec::new();
	let mut n_obj = 0;
	for raw in ing.split('\n') {
		let line = raw.trim_start_matches('\t');
		let runs = if line.contains('$') { Vec::new() } else { find_runs(line) };
		if runs.is_empty() {
			script.push(Value::String(line.to_string()));
			continue;
		}
		let mut code = String::new();
		let mut pos = 0;
		let mut texts = Vec::new();
		let mut lns = Vec::new();
		for (n, r) in runs.iter().enumerate() {
			code.push_str(&line[pos..r.a]);
			code.push_str(&format!("${}", n + 1));
			pos = r.b;
			texts.push(Value::String(pieces_to_text(&r.pieces)));
			lns.push(Value::String(ln_encode(&r.prefixes)));
		}
		code.push_str(&line[pos..]);
		let mut m = Map::new();
		m.insert("code".into(), Value::String(code));
		m.insert("text".into(), Value::Array(texts));
		if lns.iter().any(|v| v.as_str().map(|s| s.chars().any(|c| c != ',')).unwrap_or(false)) {
			m.insert("ln".into(), Value::Array(lns));
		}
		script.push(Value::Object(m));
		n_obj += 1;
	}
	let mut root = Map::new();
	root.insert("kiseki".into(), Value::String(MAGIC.into()));
	root.insert(
		tr("_справка", "_help").into(),
		Value::String(tr("Полный скрипт сцены. Переводите строки в \"text\": \\n — новая строка в окне, {11}{1180} и другие {числа} — служебные коды (оставляйте на месте), теги <...> тоже не трогайте. $1, $2 в \"code\" — места, куда подставляется текст. Поле \"ln\" — служебное.", "The full script of the scene. Translate the strings in \"text\": \\n is a new line in the window, {11}{1180} and other {numbers} are service codes (keep them in place), and don't touch <...> tags either. $1, $2 in \"code\" are places where the text goes. The \"ln\" field is internal.").into()),
	);
	root.insert("script".into(), Value::Array(script));
	(serde_json::to_string_pretty(&Value::Object(root)).unwrap(), n_obj)
}

fn json_to_ing(src: &str) -> Result<String, String> {
	let src = src.strip_prefix('\u{FEFF}').unwrap_or(src);
	let v: Value = serde_json::from_str(src).map_err(|e| lformat!("ошибка JSON (строка {}, столбец {}): {}", "JSON error (line {}, column {}): {}", e.line(), e.column(), e.to_string().split(" at line").next().unwrap_or("")))?;
	let script = v.get("script").and_then(|s| s.as_array()).ok_or(tr("нет списка \"script\"", "no \"script\" list"))?;
	let mut out = String::new();
	let mut depth: usize = 0;
	for (n, item) in script.iter().enumerate() {
		let line = match item {
			Value::String(s) => s.clone(),
			Value::Object(m) => {
				let code = m.get("code").and_then(|c| c.as_str()).ok_or_else(|| lformat!("элемент {}: нет \"code\"", "element {}: no \"code\"", n + 1))?;
				let texts = m.get("text").and_then(|t| t.as_array()).ok_or_else(|| lformat!("элемент {}: нет \"text\"", "element {}: no \"text\"", n + 1))?;
				let lns: Vec<String> = m.get("ln").and_then(|l| l.as_array()).map(|a| a.iter().map(|x| x.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default();
				let mut line = String::new();
				let mut rest = code;
				let mut used = 0;
				while let Some(p) = rest.find('$') {
					line.push_str(&rest[..p]);
					let digits: String = rest[p + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
					let k: usize = digits.parse().ok().filter(|&k| k >= 1).ok_or_else(|| lformat!("элемент {}: неверная метка $ в \"code\"", "element {}: invalid $ label in \"code\"", n + 1))?;
					let t = texts.get(k - 1).and_then(|t| t.as_str()).ok_or_else(|| lformat!("элемент {}: нет текста ${k}", "element {}: no text ${k}", n + 1))?;
					let pieces = text_to_pieces(t).map_err(|e| lformat!("текст «{}»: {e}", "text '{}': {e}", t.chars().take(40).collect::<String>()))?;
					let pref = lns.get(k - 1).map(|s| ln_decode(s));
					line.push_str(&pieces_to_src(&pieces, pref.as_deref()));
					rest = &rest[p + 1 + digits.len()..];
					used += 1;
				}
				line.push_str(rest);
				let _ = used;
				line
			}
			_ => return Err(lformat!("элемент {}: ожидалась строка или объект", "element {}: expected a string or an object", n + 1)),
		};
		let toks = lex(&line);
		let opens = toks.iter().filter(|t| t.t == T::Open && &line[t.a..t.b] == "{").count();
		let closes = toks.iter().filter(|t| t.t == T::Close && &line[t.a..t.b] == "}").count();
		let starts_close = line.starts_with('}');
		let ind = if starts_close { depth.saturating_sub(1) } else { depth };
		if n > 0 { out.push('\n') }
		if !line.is_empty() {
			out.push_str(&"\t".repeat(ind));
		}
		out.push_str(&line);
		depth = (depth + opens).saturating_sub(closes);
	}
	Ok(out)
}

pub fn decompile(data: &[u8]) -> Result<(String, usize), String> {
	let ing = to_ing(data)?;
	let (json, n) = ing_to_json(&ing);
	let ing2 = json_to_ing(&json)?;
	let a = from_ing(&ing).map_err(|e| lformat!("самопроверка (исходный скрипт): {e}", "self-check (source script): {e}"))?;
	let b = from_ing(&ing2).map_err(|e| lformat!("самопроверка (из JSON): {e}", "self-check (from JSON): {e}"))?;
	if a != b {
		if let Ok(d) = std::env::var("KAI_DEBUG") {
			let _ = std::fs::write(format!("{d}.a.ing"), &ing);
			let _ = std::fs::write(format!("{d}.b.ing"), &ing2);
		}
		return Err(tr("внутренняя ошибка: JSON собирается иначе, чем исходный скрипт", "internal error: the JSON builds differently from the source script").into());
	}
	Ok((json, n))
}

pub fn compile(json: &str) -> Result<Vec<u8>, String> {
	let ing = json_to_ing(json)?;
	from_ing(&ing)
}

pub fn looks_like(src: &str) -> bool {
	let s = src.trim_start_matches('\u{FEFF}').trim_start();
	s.starts_with('{') && s.contains("\"kai-scena\"")
}
