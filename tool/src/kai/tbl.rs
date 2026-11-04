#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use std::collections::HashMap;
use rust_xlsxwriter::{Color, Format, FormatAlign, Workbook};
use serde_json::Value;

pub const SHEET: &str = "Text";
pub const LEGACY_SHEET: &str = "\u{0422}\u{0435}\u{043a}\u{0441}\u{0442}";
const INFO_SHEET: &str = "_internal";
const MAGIC: &str = "kiseki-tool kai tbl 1";

static SCHEMAS: &str = include_str!("tbl_schemas.json");

struct Header {
	name: String,
	start: usize,
	len: usize,
	count: usize,
}

fn u32at(d: &[u8], o: usize) -> Result<usize, String> {
	d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize).ok_or_else(|| tr("файл обрезан", "the file is truncated").to_string())
}
fn u64at(d: &[u8], o: usize) -> Result<u64, String> {
	d.get(o..o + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| tr("файл обрезан", "the file is truncated").to_string())
}

fn headers(d: &[u8]) -> Result<Vec<Header>, String> {
	if !d.starts_with(b"#TBL") {
		return Err(tr("это не таблица #TBL (возможно, файл зашифрован)", "this is not a #TBL table (the file may be encrypted)").into());
	}
	let n = u32at(d, 4)?;
	let mut out = Vec::new();
	let mut o = 8;
	for _ in 0..n {
		let name_raw = d.get(o..o + 64).ok_or(tr("файл обрезан", "the file is truncated"))?;
		let end = name_raw.iter().position(|&b| b == 0).unwrap_or(64);
		out.push(Header {
			name: String::from_utf8_lossy(&name_raw[..end]).into_owned(),
			start: u32at(d, o + 68)?,
			len: u32at(d, o + 72)?,
			count: u32at(d, o + 76)?,
		});
		o += 80;
	}
	Ok(out)
}

fn data_end(hs: &[Header]) -> usize {
	hs.iter().map(|h| h.start.saturating_add(h.len.saturating_mul(h.count))).max().unwrap_or(0)
}

fn with_schemas<R>(f: impl FnOnce(&Value) -> R) -> R {
	thread_local! {
		static DB: Value = serde_json::from_str(SCHEMAS).unwrap();
	}
	DB.with(f)
}

fn text_fields(name: &str, len: usize) -> Option<Vec<(usize, String)>> {
	with_schemas(|db| {
		let list = db.get(name)?.as_array()?;
		let pick = list
			.iter()
			.find(|s| s["size"].as_u64() == Some(len as u64) && s["game"].as_str().map(|g| g.starts_with(super::game::current().schema_tag())).unwrap_or(false))
			.or_else(|| list.iter().find(|s| s["size"].as_u64() == Some(len as u64)))?;
		Some(
			pick["text"]
				.as_array()?
				.iter()
				.filter_map(|t| Some((t[0].as_u64()? as usize, t[1].as_str()?.to_string())))
				.collect(),
		)
	})
}

fn known_text_section(name: &str) -> bool {
	with_schemas(|db| {
		db.get(name).and_then(|l| l.as_array()).map(|l| l.iter().any(|s| s["text"].as_array().map(|t| !t.is_empty()).unwrap_or(false))).unwrap_or(false)
	})
}

fn fields_for(d: &[u8], h: &Header, data_end: usize) -> Option<Vec<(usize, String)>> {
	if let Some(f) = text_fields(&h.name, h.len) {
		return Some(f);
	}
	if h.count == 0 || h.len < 8 || !known_text_section(&h.name) {
		return None;
	}
	let mut out = Vec::new();
	let mut off = 0;
	while off + 8 <= h.len {
		let mut ok = true;
		let mut any_text = false;
		for e in 0..h.count {
			let pos = h.start + e * h.len + off;
			let Ok(p) = u64at(d, pos) else { ok = false; break };
			let p = p as usize;
			if p < data_end || p >= d.len() {
				ok = false;
				break;
			}
			match read_cstr(d, p) {
				Some(s) => {
					if !s.is_empty() && !identifier(s) && !s.contains('/') && (s.contains(' ') || !s.is_ascii()) {
						any_text = true
					}
				}
				None => {
					ok = false;
					break;
				}
			}
		}
		if ok && any_text {
			out.push((off, format!("field{off}")));
		}
		off += 8;
	}
	(!out.is_empty()).then_some(out)
}

fn read_cstr(d: &[u8], p: usize) -> Option<&str> {
	let rest = d.get(p..)?;
	let end = rest.iter().position(|&b| b == 0)?;
	let s = std::str::from_utf8(&rest[..end]).ok()?;
	if s.chars().any(|c| c.is_control() && c != '\n') {
		return None;
	}
	Some(s)
}

fn identifier(s: &str) -> bool {
	s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || c == b'-')
		&& (s.contains('_') || (s.bytes().any(|c| c.is_ascii_lowercase()) && s.bytes().any(|c| c.is_ascii_digit())))
}

pub struct Row {
	pub id: String,
	pub text: String,
	pub ptr_pos: usize,
}

pub fn rows(d: &[u8]) -> Result<(Vec<Row>, Vec<String>), String> {
	let hs = headers(d)?;
	let data_end = data_end(&hs);
	let mut out = Vec::new();
	let mut missing = Vec::new();
	for h in &hs {
		let Some(fields) = fields_for(d, h, data_end) else {
			missing.push(h.name.clone());
			continue;
		};
		for e in 0..h.count {
			for (off, fname) in &fields {
				let pos = h.start + e * h.len + off;
				let Ok(p) = u64at(d, pos) else { continue };
				let p = p as usize;
				if p < data_end || p >= d.len() {
					continue;
				}
				let Some(s) = read_cstr(d, p) else { continue };
				if s.is_empty() || identifier(s) {
					continue;
				}
				out.push(Row { id: format!("{}#{}.{}", h.name, e, fname), text: s.to_string(), ptr_pos: pos });
			}
		}
	}
	let tech = technical_fields(&out);
	out.retain(|r| !tech.contains(&field_key(&r.id)));
	Ok((out, missing))
}

fn field_key(id: &str) -> String {
	match (id.find('#'), id.rfind('.')) {
		(Some(h), Some(d)) if d > h => format!("{}.{}", &id[..h], &id[d + 1..]),
		_ => id.to_string(),
	}
}

fn technical_set(rows: &[(String, String)]) -> std::collections::HashSet<String> {
	let mut by: HashMap<String, (usize, usize)> = HashMap::new();
	for (id, text) in rows {
		let k = field_key(id);
		let e = by.entry(k).or_insert((0, 0));
		e.0 += 1;
		let ascii = !text.is_empty() && text.len() <= 32 && text.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || c == b'-');
		let ordinal = ["st", "nd", "rd", "th"].iter().any(|e| text.len() > e.len() && text.ends_with(e) && text[..text.len() - e.len()].bytes().all(|c| c.is_ascii_digit()));
		let code = ascii && !ordinal
			&& (text.bytes().all(|c| !c.is_ascii_uppercase())
				|| text.contains('_')
				|| text.contains('.')
				|| (text.len() <= 4 && text.bytes().all(|c| c.is_ascii_uppercase())));
		if code {
			e.1 += 1;
		}
	}
	by.into_iter()
		.filter(|(k, (n, tok))| {
			let f = k.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
			let by_name = ["model", "face", "script", "animation", "ai_file", "motion"].contains(&f.as_str()) || f.starts_with("file");
			by_name || (*n >= 3 && *tok * 10 >= *n * 9)
		})
		.map(|(k, _)| k)
		.collect()
}

fn technical_fields(rows: &[Row]) -> std::collections::HashSet<String> {
	let v: Vec<(String, String)> = rows.iter().map(|r| (r.id.clone(), r.text.clone())).collect();
	technical_set(&v)
}

pub fn decompile(d: &[u8], file_name: &str) -> Result<(Vec<u8>, usize, Vec<String>), String> {
	let (rows, missing) = rows(d)?;
	let book = write_book(&rows, file_name)?;
	let back = compile(&book, d)?;
	if back != d {
		return Err(tr("внутренняя ошибка: без правок таблица собирается иначе", "internal error: without edits the table builds differently").into());
	}
	Ok((book, rows.len(), missing))
}

fn write_book(rows: &[Row], file_name: &str) -> Result<Vec<u8>, String> {
	let e = |x: rust_xlsxwriter::XlsxError| lformat!("ошибка создания Excel: {x}", "Excel creation error: {x}");
	let mut wb = Workbook::new();
	let head = Format::new().set_bold().set_background_color(Color::RGB(0xDDEBF7));
	let wrap = Format::new().set_text_wrap().set_align(FormatAlign::Top);
	let grey = Format::new().set_text_wrap().set_align(FormatAlign::Top).set_font_color(Color::RGB(0x808080));
	{
		let ws = wb.add_worksheet();
		ws.set_name(SHEET).map_err(e)?;
		ws.write_string_with_format(0, 0, tr("ID (раздел#номер.поле)", "ID (section#number.field)"), &head).map_err(e)?;
		ws.write_string_with_format(0, 1, &lformat!("Текст — редактируйте этот столбец ({file_name})", "Text: edit this column ({file_name})"), &head).map_err(e)?;
		ws.write_string_with_format(0, 2, tr("Оригинал (для справки)", "Original (for reference)"), &head).map_err(e)?;
		ws.write_string_with_format(0, 3, tr("служебное", "internal"), &head).map_err(e)?;
		ws.set_column_width(0, 34).map_err(e)?;
		ws.set_column_width(1, 70).map_err(e)?;
		ws.set_column_width(2, 60).map_err(e)?;
		ws.set_column_hidden(3).map_err(e)?;
		ws.set_freeze_panes(1, 0).map_err(e)?;
		for (i, r) in rows.iter().enumerate() {
			let n = i as u32 + 1;
			ws.write_string_with_format(n, 0, &r.id, &grey).map_err(e)?;
			ws.write_string_with_format(n, 1, &r.text, &wrap).map_err(e)?;
			ws.write_string_with_format(n, 2, &r.text, &grey).map_err(e)?;
			ws.write_string(n, 3, &r.ptr_pos.to_string()).map_err(e)?;
		}
	}
	{
		let ws = wb.add_worksheet();
		ws.set_name(INFO_SHEET).map_err(e)?;
		ws.write_string(0, 0, MAGIC).map_err(e)?;
		ws.write_string(0, 1, file_name).map_err(e)?;
		ws.set_hidden(true);
	}
	wb.save_to_buffer().map_err(e)
}

fn cell(d: &calamine::Data) -> String {
	use calamine::Data;
	match d {
		Data::Empty => String::new(),
		Data::String(s) => s.clone(),
		Data::Float(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", *f as i64),
		other => other.to_string(),
	}
}

pub fn read_book(book: &[u8]) -> Result<Vec<(String, String, usize, usize)>, String> {
	use calamine::{Reader, Xlsx};
	let mut wb: Xlsx<_> = calamine::open_workbook_from_rs(std::io::Cursor::new(book.to_vec())).map_err(|e| lformat!("не удалось открыть Excel: {e}", "could not open Excel: {e}"))?;
	let ws = wb.worksheet_range(SHEET).or_else(|_| wb.worksheet_range(LEGACY_SHEET)).map_err(|_| lformat!("нет листа «{SHEET}»", "no '{SHEET}' sheet"))?;
	let mut out = Vec::new();
	for (r, row) in ws.rows().enumerate().skip(1) {
		let id = row.first().map(cell).unwrap_or_default();
		if id.trim().is_empty() {
			continue;
		}
		let text = row.get(1).map(cell).unwrap_or_default().replace("\r\n", "\n");
		let pos: usize = row.get(3).map(cell).unwrap_or_default().trim().parse().map_err(|_| lformat!("строка {}: повреждён служебный столбец D", "line {}: service column D is damaged", r + 1))?;
		out.push((id, text, pos, r + 1));
	}
	Ok(out)
}

pub fn compile(book: &[u8], orig: &[u8]) -> Result<Vec<u8>, String> {
	compile_checked(book, orig).map(|x| x.0)
}

pub fn compile_checked(book: &[u8], orig: &[u8]) -> Result<(Vec<u8>, Vec<String>), String> {
	let rows = read_book(book)?;
	let mut fields_at: HashMap<usize, String> = HashMap::new();
	let tech: std::collections::HashSet<String> = {
		let mut all: Vec<(String, String)> = Vec::new();
		if let Ok(hs) = headers(orig) {
			let data_end = data_end(&hs);
			for h in &hs {
				let Some(fields) = fields_for(orig, h, data_end) else { continue };
				for e in 0..h.count {
					for (off, fname) in &fields {
						let pos = h.start + e * h.len + off;
						let id = format!("{}#{}.{}", h.name, e, fname);
						fields_at.insert(pos, id.clone());
						let Ok(p) = u64at(orig, pos) else { continue };
						let p = p as usize;
						if p < data_end || p >= orig.len() {
							continue;
						}
						if let Some(s) = read_cstr(orig, p) {
							if !s.is_empty() {
								all.push((id, s.to_string()));
							}
						}
					}
				}
			}
		}
		technical_set(&all)
	};
	let mut skipped = Vec::new();
	let mut out = orig.to_vec();
	let mut added: HashMap<String, u64> = HashMap::new();
	for (id, text, pos, r) in rows {
		if fields_at.get(&pos) != Some(&id) {
			return Err(lformat!("строка {r} ({id}): книга не подходит к этой таблице (или испорчен служебный столбец D) — декомпилируйте заново", "line {r} ({id}): the workbook doesn't fit this table (or service column D is damaged), decompile again"));
		}
		let p = u64at(orig, pos).map_err(|_| lformat!("строка {r} ({id}): неверная позиция", "line {r} ({id}): invalid position"))? as usize;
		let old = read_cstr(orig, p).ok_or_else(|| lformat!("строка {r} ({id}): исходная таблица не совпадает с книгой — декомпилируйте заново", "line {r} ({id}): the source table doesn't match the workbook, decompile again"))?;
		if old == text {
			continue;
		}
		if tech.contains(&field_key(&id)) {
			skipped.push(format!("{id}: «{old}» -> «{text}»"));
			continue;
		}
		if text.contains('\0') {
			return Err(lformat!("строка {r} ({id}): недопустимый символ", "line {r} ({id}): invalid character"));
		}
		let new_p = match added.get(&text) {
			Some(&p) => p,
			None => {
				let p = out.len() as u64;
				out.extend_from_slice(text.as_bytes());
				out.push(0);
				added.insert(text.clone(), p);
				p
			}
		};
		out[pos..pos + 8].copy_from_slice(&new_p.to_le_bytes());
	}
	Ok((out, skipped))
}
