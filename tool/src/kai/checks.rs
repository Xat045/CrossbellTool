#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use std::collections::HashMap;

use super::compare::Data;

pub const CAT_KEYS: [&str; 3] = ["\u{0421}\u{043b}\u{0443}\u{0436}\u{0435}\u{0431}\u{043d}\u{044b}\u{0435} \u{043a}\u{043e}\u{0434}\u{044b}", "\u{041d}\u{0435} \u{043f}\u{0435}\u{0440}\u{0435}\u{0432}\u{0435}\u{0434}\u{0435}\u{043d}\u{043e}", "\u{041f}\u{043e}\u{0434}\u{0441}\u{0442}\u{0430}\u{043d}\u{043e}\u{0432}\u{043a}\u{0438}"];

pub fn cats() -> [(&'static str, &'static str, u8); 3] {
	[
	(tr("Служебные коды", "Service codes"), tr("Цвет <C…>/</C>, коды {11}{…}, {16}, {7}, теги <K>, <E…>, <B>… не совпадают с оригиналом. И сломанный перенос: \\ без n.", "Color <C…>/</C>, codes {11}{…}, {16}, {7}, tags <K>, <E…>, <B>… don't match the original. Also a broken line break: \\ without n."), 0),
	(tr("Не переведено", "Not translated"), tr("Реплика сцены осталась без перевода целиком или частично (английский или японский текст). Для пары японский и английский ищется японский текст в английских файлах. Таблицы и интерфейс не проверяются. Одинаковые тексты собраны в одну строку, в скобках — сколько раз.", "A scene line is left untranslated, fully or partly (English or Japanese text). For a Japanese and English pair, Japanese text is searched for in the English files. Tables and the interface are not checked. Identical texts are collected in one line, with the count in brackets."), 1),
	(tr("Подстановки", "Placeholders"), tr("%d, %s, %.1f… — места, куда игра подставляет число или имя: в переводе их не столько же, сколько в оригинале. Игра покажет не то или упадёт.", "%d, %s, %.1f… are places where the game inserts a number or a name: the translation does not have as many as the original. The game will show the wrong thing or crash."), 0),
]
}

pub fn cat_desc(i: usize) -> &'static str {
	if i == CAT_CODES && super::compare::ed7().is_some() {
		tr("Коды не совпадают с оригиналом: {wait}, {color N}, {0x06}, {0x0D}, {item[…]}, {}, лица #…F, голос #…V, #…P, #…W, #…S, значки #…I/#…M. Особые знаки (♥ ♪ ★ ━ ∴ ※ …) потеряны или лишние. И сломанный текст: «\\» посреди строки, обрывки кодов (x01).", "Codes don't match the original: {wait}, {color N}, {0x06}, {0x0D}, {item[…]}, {}, faces #…F, voice #…V, #…P, #…W, #…S, icons #…I/#…M. Special signs (♥ ♪ ★ ━ ∴ ※ …) are lost or extra. Also broken text: a '\\' in the middle of a line, code fragments (x01).")
	} else {
		cats()[i].1
	}
}

pub const CAT_CODES: usize = 0;
pub const CAT_UNTR: usize = 1;
pub const CAT_FMT: usize = 2;

pub const NO_PAIR: usize = usize::MAX;

#[derive(Clone, Debug)]
pub struct Finding {
	pub cat: usize,
	pub pair: usize,
	pub what: String,
}

#[derive(Default)]
pub struct Report {
	pub findings: Vec<Finding>,
	pub checked: usize,
}

fn token_at_ed7(c: &[char], i: usize) -> Option<(String, usize)> {
	match c[i] {
		'{' => {
			let j = c[i + 1..].iter().take(24).position(|&x| x == '}')?;
			let inner: String = c[i + 1..i + 1 + j].iter().collect();
			if inner.chars().any(|x| x == '{' || x == '\n') {
				return None;
			}
			let norm: String = inner.split_whitespace().collect::<Vec<_>>().join(" ");
			Some((format!("{{{norm}}}"), j + 2))
		}
		'#' => {
			let mut j = i + 1;
			while j < c.len() && c[j].is_ascii_digit() {
				j += 1;
			}
			let letter = *c.get(j)?;
			if !letter.is_ascii_alphabetic() {
				return None;
			}
			let num: String = c[i + 1..j].iter().collect();
			let num = num.trim_start_matches('0');
			let num = if num.is_empty() && j > i + 1 { "0" } else { num };
			Some((format!("#{num}{letter}"), j + 1 - i))
		}
		_ => None,
	}
}

fn token_at(c: &[char], i: usize) -> Option<(String, usize)> {
	if super::compare::ed7().is_some() {
		return token_at_ed7(c, i);
	}
	let close = match c[i] {
		'{' => '}',
		'<' => '>',
		_ => return None,
	};
	let j = c[i + 1..].iter().take(40).position(|&x| x == close)?;
	let inner: String = c[i + 1..i + 1 + j].iter().filter(|x| !x.is_whitespace()).collect();
	let ok = if close == '}' { inner == "tab" || (!inner.is_empty() && inner.chars().all(|x| x.is_ascii_digit())) } else { !inner.is_empty() && inner.chars().all(|x| x.is_ascii_alphanumeric() || x == '/' || x == '#' || x == '_') };
	ok.then(|| (format!("{}{}{}", c[i], inner, close), j + 2))
}

fn tokens(s: &str) -> Vec<String> {
	let c: Vec<char> = s.chars().collect();
	let mut out = Vec::new();
	let mut i = 0;
	while i < c.len() {
		match token_at(&c, i) {
			Some((t, len)) => {
				out.push(t);
				i += len;
			}
			None => i += 1,
		}
	}
	out
}

fn jp_free(v: Vec<String>) -> Vec<String> {
	let typo = |t: &str| {
		let low_num = |p: &str| t.strip_prefix(p).and_then(|r| r.strip_suffix('>')).is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()));
		t == "<R>" || t.starts_with("</R") || t == "<B>" || t == "</B>" || t == "</C>" || low_num("<s") || low_num("<c")
	};
	v.into_iter().filter(|t| !typo(t)).collect()
}

fn strip(s: &str) -> String {
	let c: Vec<char> = s.chars().collect();
	let mut out = String::with_capacity(s.len());
	let mut i = 0;
	while i < c.len() {
		match token_at(&c, i) {
			Some((_, len)) => i += len,
			None => {
				out.push(c[i]);
				i += 1;
			}
		}
	}
	out
}

fn count(v: Vec<String>) -> HashMap<String, usize> {
	let mut m = HashMap::new();
	for x in v {
		*m.entry(x).or_insert(0) += 1;
	}
	m
}

fn diff(a: &HashMap<String, usize>, b: &HashMap<String, usize>) -> Vec<String> {
	let mut v: Vec<String> = a.iter().filter_map(|(k, &n)| (n > *b.get(k).unwrap_or(&0)).then(|| k.clone())).collect();
	v.sort();
	v
}

fn has_jp(s: &str) -> bool {
	s.chars().any(|c| ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c))
}

fn is_dialogue(file: &str) -> bool {
	if super::compare::ed7().is_some() { file.starts_with("scena/") } else { file.ends_with(".json") }
}

fn has_cyr(s: &str) -> bool {
	s.chars().any(|c| ('а'..='я').contains(&c) || ('А'..='Я').contains(&c) || c == 'ё' || c == 'Ё')
}

fn latin_words(s: &str) -> Vec<String> {
	let mut out = Vec::new();
	let mut cur = String::new();
	for ch in s.chars().chain(std::iter::once(' ')) {
		if ch.is_ascii_alphabetic() || (ch == '-' && !cur.is_empty()) {
			cur.push(ch);
		} else {
			let w = cur.trim_end_matches('-').to_string();
			if w.chars().filter(|c| c.is_ascii_alphabetic()).count() >= 3 {
				out.push(w);
			}
			cur.clear();
		}
	}
	out
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Target {
	Russian,
	English,
	Japanese,
}

fn target_lang(d: &Data) -> Target {
	let (mut total, mut src_jp, mut tgt_jp, mut tgt_cyr) = (0usize, 0usize, 0usize, 0usize);
	for p in &d.pairs {
		let (Some(en), Some(ru)) = (&p.en, &p.ru) else { continue };
		if p.service || is_debug(&p.file) || !is_dialogue(&p.file) {
			continue;
		}
		total += 1;
		src_jp += has_jp(&strip(&en.text)) as usize;
		tgt_jp += has_jp(&strip(&ru.text)) as usize;
		tgt_cyr += has_cyr(&ru.text) as usize;
	}
	if tgt_cyr > 0 && tgt_cyr * 100 >= total {
		return Target::Russian;
	}
	match (src_jp * 2 > total, tgt_jp * 2 > total) {
		(true, false) => Target::English,
		(false, true) => Target::Japanese,
		_ => Target::Russian,
	}
}

fn leftover_jp(s: &str) -> Option<String> {
	let jp: String = s.chars().filter(|c| ('\u{3041}'..='\u{3096}').contains(c) || ('\u{30a1}'..='\u{30fa}').contains(c) || ('\u{4e00}'..='\u{9fff}').contains(c)).collect();
	(jp.chars().count() >= 2).then_some(jp)
}

fn leftover(s: &str) -> Option<String> {
	if let Some(jp) = leftover_jp(s) {
		return Some(jp);
	}
	let mut run: Vec<&str> = Vec::new();
	let upper = |w: &&str| w.starts_with(|c: char| c.is_ascii_uppercase());
	let good = |run: &[&str]| run.len() >= 3 && run.iter().filter(|w| w.len() >= 2 && w.starts_with(|c: char| c.is_ascii_lowercase())).count() >= 2 && !(upper(&run[0]) && upper(&run[run.len() - 1]));
	for tok in s.split_whitespace() {
		let w = tok.trim_matches(|c: char| !c.is_alphanumeric());
		let latin = !w.is_empty() && w.chars().all(|c| c.is_ascii_alphabetic() || c == '\'' || c == '’' || c == '-');
		if latin {
			run.push(w);
			continue;
		}
		if good(&run) {
			return Some(run.join(" "));
		}
		run.clear();
	}
	good(&run).then(|| run.join(" "))
}

pub fn specs(t: &str) -> Vec<String> {
	let c: Vec<char> = t.chars().collect();
	let mut out = Vec::new();
	let mut i = 0;
	while i < c.len() {
		if c[i] != '%' {
			i += 1;
			continue;
		}
		if c.get(i + 1) == Some(&'%') {
			i += 2;
			continue;
		}
		let mut j = i + 1;
		while j < c.len() && "-+#0123456789.lh".contains(c[j]) {
			j += 1;
		}
		let glued = c.get(j + 1).map(|x| x.is_ascii_lowercase()).unwrap_or(false);
		match c.get(j) {
			Some(&x) if "diouxXeEfFgGcsp".contains(x) && !glued => {
				out.push(c[i..=j].iter().collect());
				i = j + 1;
			}
			_ => i += 1,
		}
	}
	out.sort();
	out
}

fn is_debug(file: &str) -> bool {
	let name = file.rsplit('/').next().unwrap_or(file).to_lowercase();
	let b = name.as_bytes();
	name.contains("debug") || name.contains("sample") || (b.len() > 1 && b[0] == b'a' && b[1].is_ascii_digit())
}

fn broken_newline(en: &str, ru: &str) -> Option<String> {
	let cnt = |s: &str| {
		let c: Vec<char> = s.chars().collect();
		let (mut bare, mut text_n) = (0, 0);
		for (k, &ch) in c.iter().enumerate() {
			if ch == '\\' {
				if c.get(k + 1) == Some(&'n') { text_n += 1 } else { bare += 1 }
			}
		}
		(bare, text_n)
	};
	let (eb, en_n) = cnt(en);
	let (rb, ru_n) = cnt(ru);
	if rb > eb {
		Some(tr("\\ без n — диалог дальше не пойдёт", "\\ without n: the dialogue won't continue").to_string())
	} else if ru_n > en_n {
		Some(tr("\\n текстом — вместо переноса в игре будет «\\n»", "\\n as text: the game will show '\\n' instead of a line break").to_string())
	} else {
		None
	}
}

fn broken_ed7(en: &str, ru: &str) -> Vec<String> {
	let cnt_bs = |s: &str| {
		let c: Vec<char> = s.chars().collect();
		(0..c.len()).filter(|&k| c[k] == '\\' && !matches!(c.get(k + 1), None | Some('\n') | Some('\r'))).count()
	};
	let cnt_x = |s: &str| {
		let c: Vec<char> = s.chars().collect();
		(1..c.len().saturating_sub(2)).filter(|&k| c[k] == 'x' && c[k - 1].is_alphabetic() && c[k + 1].is_ascii_hexdigit() && c[k + 2].is_ascii_hexdigit() && c[k + 1] <= '1').count()
	};
	let mut v = Vec::new();
	if cnt_bs(ru) > cnt_bs(en) {
		v.push(tr("«\\» посреди строки — в игре будет виден", "a '\\' in the middle of a line will be visible in the game").to_string());
	}
	if cnt_x(ru) > cnt_x(en) {
		v.push(tr("обрывок кода (x01…) — вероятно, сломанный {x01}", "code fragment (x01…), probably a broken {x01}").to_string());
	}
	v
}

const ED7_SYMBOLS: &[char] = &['㈱', '♪', '★', '☆', '◆', '━', '∴', '※', '←', '→', '①', '②', '③', 'Ⅰ', 'Ⅱ', 'Ⅲ'];

fn symbols_ed7(en: &str, ru: &str) -> Option<String> {
	let show = |c: char| match c {
		'㈱' => "♥ (㈱)".to_string(),
		'㍉' => "è (㍉)".to_string(),
		_ => c.to_string(),
	};
	let cnt = |s: &str| {
		let mut m: HashMap<char, usize> = HashMap::new();
		for c in s.chars().filter(|c| ED7_SYMBOLS.contains(c)) {
			*m.entry(c).or_default() += 1;
		}
		m
	};
	let (e, r) = (cnt(en), cnt(ru));
	if e == r {
		return None;
	}
	let mut lost = Vec::new();
	let mut extra = Vec::new();
	for &c in ED7_SYMBOLS {
		let (a, b) = (*e.get(&c).unwrap_or(&0), *r.get(&c).unwrap_or(&0));
		if a > b {
			lost.push(show(c));
		} else if b > a {
			extra.push(show(c));
		}
	}
	let mut v = Vec::new();
	if !lost.is_empty() {
		v.push(lformat!("знаки потеряны: {}", "signs lost: {}", lost.join(" ")));
	}
	if !extra.is_empty() {
		v.push(lformat!("знаки лишние: {}", "extra signs: {}", extra.join(" ")));
	}
	Some(v.join(" · "))
}

pub fn run(d: &Data) -> Report {
	let mut r = Report::default();
	let target = target_lang(d);
	for w in &d.warnings {
		r.findings.push(Finding { cat: CAT_CODES, pair: NO_PAIR, what: w.clone() });
	}
	for (i, p) in d.pairs.iter().enumerate() {
		let (Some(en), Some(ru)) = (&p.en, &p.ru) else { continue };
		if p.service {
			if ru.text.trim() != en.text.trim() {
				r.findings.push(Finding { cat: CAT_CODES, pair: i, what: tr("служебное имя изменено — игра его не найдёт", "service name changed: the game won't find it").into() });
			}
			continue;
		}
		let debug = is_debug(&p.file);
		let es = strip(&en.text);
		if latin_words(&es).is_empty() && !has_jp(&es) && !has_cyr(&es) {
			continue;
		}
		r.checked += 1;
		if super::compare::ed7().is_some() {
			for w in broken_ed7(&en.text, &ru.text) {
				r.findings.push(Finding { cat: CAT_CODES, pair: i, what: w });
			}
			if !debug && has_cyr(&ru.text) {
				if let Some(w) = symbols_ed7(&en.text, &ru.text) {
					r.findings.push(Finding { cat: CAT_CODES, pair: i, what: w });
				}
			}
		} else if let Some(w) = broken_newline(&en.text, &ru.text) {
			r.findings.push(Finding { cat: CAT_CODES, pair: i, what: w });
		}
		if !has_cyr(&ru.text) {
			if debug || !is_dialogue(&p.file) {
				continue;
			}
			let rs_clean = strip(&ru.text);
			let untranslated = match target {
				Target::Russian => ru.text.trim() == en.text.trim() || !latin_words(&rs_clean).is_empty() || has_jp(&rs_clean),
				Target::English => leftover_jp(&rs_clean).is_some(),
				Target::Japanese => !has_jp(&rs_clean) && !latin_words(&rs_clean).is_empty(),
			};
			if untranslated {
				let part = target == Target::English && !latin_words(&rs_clean).is_empty();
				r.findings.push(Finding { cat: CAT_UNTR, pair: i, what: if part { leftover_jp(&rs_clean).unwrap_or_default() } else { String::new() } });
			}
			continue;
		}
		if !debug && is_dialogue(&p.file) {
			if let Some(w) = leftover(&strip(&ru.text)) {
				r.findings.push(Finding { cat: CAT_UNTR, pair: i, what: w });
			}
		}
		let (te, tr) = if has_jp(&en.text) { (jp_free(tokens(&en.text)), jp_free(tokens(&ru.text))) } else { (tokens(&en.text), tokens(&ru.text)) };
		if let Some(w) = lost_extra(te, tr) {
			r.findings.push(Finding { cat: CAT_CODES, pair: i, what: w });
		}
		if let Some(w) = lost_extra(specs(&en.text), specs(&ru.text)) {
			r.findings.push(Finding { cat: CAT_FMT, pair: i, what: w });
		}
	}
	r
}

fn lost_extra(en: Vec<String>, ru: Vec<String>) -> Option<String> {
	let (te, tr) = (count(en), count(ru));
	if te == tr {
		return None;
	}
	let lost = diff(&te, &tr);
	let extra = diff(&tr, &te);
	let mut what = Vec::new();
	if !lost.is_empty() {
		what.push(lformat!("потеряно: {}", "missing: {}", lost.join(" ")));
	}
	if !extra.is_empty() {
		what.push(lformat!("лишнее: {}", "extra: {}", extra.join(" ")));
	}
	Some(what.join(" · "))
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn toks() {
		assert_eq!(tokens("<C1 >Hi</C >{16}"), vec!["<C1>", "</C>", "{16}"]);
		assert_eq!(strip("<C1>Hi</C>{16}"), "Hi");
		assert_eq!(strip("<C1 >Hi</C >{ 16 }"), "Hi");
		assert_eq!(broken_newline("a\\b", "а\\б"), None);
		assert!(broken_newline("a\\b", "а\\б\\n").unwrap().starts_with("\\n"));
		assert!(broken_newline("a", "а\\ б").unwrap().starts_with(tr("\\ без", "\\ without")));
		assert_eq!(latin_words("Привет, CID и tPhone-X"), vec!["CID", "tPhone-X"]);
	}

	#[test]
	fn leftovers() {
		let s = "Честно говоря, мне следовало давать им больше opportunities like this while I was still a student.";
		assert_eq!(leftover(s).as_deref(), Some("opportunities like this while I was still a student"));
		assert!(leftover("Давай найдём Ivy в баре").is_none());
		assert!(leftover("Это бар Black Rose Lounge в центре").is_none());
		assert!(leftover("Привет, CID и tPhone-X").is_none());
		assert_eq!(leftover("Он сказал 来てください и ушёл").as_deref(), Some("来てください"));
		assert!(leftover("Не пропустите! ♪ ヽ(*ﾟ∀ﾟ)ﾉ").is_none());
		assert!(leftover("Он замолчал ・・・・ и ушёл").is_none());
		assert!(leftover("Книга Trails in the Database на полке").is_none());
	}

	#[test]
	fn new_checks() {
		assert_eq!(specs("Got %d items from %s (100%, 50% урона, %%)"), vec!["%d", "%s"]);
		assert_eq!(specs("%.1f%% and %2d"), vec!["%.1f", "%2d"]);
		assert!(specs("up 100% to 200% total, %done").is_empty());
		assert_eq!(lost_extra(specs("%d of %d"), specs("%d из")), Some(tr("потеряно: %d", "missing: %d").into()));
		assert_eq!(lost_extra(specs("%s"), specs("%s")), None);
	}
}
