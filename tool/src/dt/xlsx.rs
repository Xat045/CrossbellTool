#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use std::collections::HashMap;
use rust_xlsxwriter::{Color, Format, FormatAlign, Workbook};
use super::model::{self, Doc};

pub const TEXT_SHEET: &str = "Text";
pub const DATA_SHEET: &str = "_internal";
const LEGACY_TEXT_SHEET: &str = "\u{0422}\u{0435}\u{043a}\u{0441}\u{0442}";
const LEGACY_DATA_SHEET: &str = "_\u{0441}\u{043b}\u{0443}\u{0436}\u{0435}\u{0431}\u{043d}\u{043e}\u{0435}";
pub const MAGIC: &str = "kiseki-tool table xlsx 1";
const CHUNK: usize = 30000;

pub fn write(doc: &Doc, file_name: &str, format_name: &str) -> Result<Vec<u8>, String> {
	let mut rows = Vec::new();
	for i in 0..doc.items.len() {
		if let Some(t) = doc.item_text(i) {
			rows.push((format!("s{i}"), t.clone(), t));
		}
	}
	let mut data = Vec::new();
	for i in 0..doc.items.len() {
		let h = doc.item_header(i);
		let chars: Vec<char> = h.chars().collect();
		for (n, chunk) in chars.chunks(CHUNK).enumerate() {
			data.push((chunk.iter().collect::<String>(), n > 0));
		}
	}
	write_raw(&rows, &data, file_name, format_name)
}

fn write_raw(rows: &[(String, String, String)], data: &[(String, bool)], file_name: &str, format_name: &str) -> Result<Vec<u8>, String> {
	let e = |x: rust_xlsxwriter::XlsxError| lformat!("ошибка создания Excel: {x}", "Excel creation error: {x}");
	let mut wb = Workbook::new();

	let head = Format::new().set_bold().set_background_color(Color::RGB(0xDDEBF7));
	let wrap = Format::new().set_text_wrap().set_align(FormatAlign::Top);
	let id = Format::new().set_align(FormatAlign::Top).set_font_color(Color::RGB(0x808080));
	let orig = Format::new().set_text_wrap().set_align(FormatAlign::Top).set_font_color(Color::RGB(0x808080));

	{
		let ws = wb.add_worksheet();
		ws.set_name(TEXT_SHEET).map_err(e)?;
		ws.write_string_with_format(0, 0, "ID", &head).map_err(e)?;
		ws.write_string_with_format(0, 1, &lformat!("Текст — редактируйте этот столбец ({file_name})", "Text: edit this column ({file_name})"), &head).map_err(e)?;
		ws.write_string_with_format(0, 2, tr("Оригинал (для справки, при сборке не используется)", "Original (for reference, not used when building)"), &head).map_err(e)?;
		ws.set_column_width(0, 8).map_err(e)?;
		ws.set_column_width(1, 70).map_err(e)?;
		ws.set_column_width(2, 70).map_err(e)?;
		ws.set_freeze_panes(1, 0).map_err(e)?;
		for (n, (i, t, o)) in rows.iter().enumerate() {
			if t.chars().count() > 32000 || o.chars().count() > 32000 {
				return Err(tr("строка длиннее 32000 символов не помещается в ячейку Excel", "a string longer than 32000 characters does not fit an Excel cell").into());
			}
			let r = n as u32 + 1;
			ws.write_string_with_format(r, 0, i, &id).map_err(e)?;
			ws.write_string_with_format(r, 1, t, &wrap).map_err(e)?;
			ws.write_string_with_format(r, 2, o, &orig).map_err(e)?;
		}
	}
	{
		let ws = wb.add_worksheet();
		ws.set_name(DATA_SHEET).map_err(e)?;
		ws.write_string(0, 0, MAGIC).map_err(e)?;
		ws.write_string(0, 1, file_name).map_err(e)?;
		ws.write_string(0, 2, format_name).map_err(e)?;
		for (n, (c, cont)) in data.iter().enumerate() {
			let r = n as u32 + 1;
			ws.write_string(r, 0, c).map_err(e)?;
			if *cont {
				ws.write_string(r, 1, "+").map_err(e)?;
			}
		}
		ws.set_hidden(true);
	}
	wb.save_to_buffer().map_err(e)
}

fn cell_to_string(d: &calamine::Data) -> String {
	use calamine::Data;
	match d {
		Data::Empty => String::new(),
		Data::String(s) => s.clone(),
		Data::Float(f) => {
			if f.fract() == 0.0 && f.abs() < 1e15 { format!("{}", *f as i64) } else { format!("{f}") }
		}
		Data::Int(i) => i.to_string(),
		Data::Bool(b) => if *b { "TRUE".into() } else { "FALSE".into() },
		other => other.to_string(),
	}
}

pub fn to_table_text(data: &[u8]) -> Result<String, String> {
	use calamine::{Reader, Xlsx};
	let mut wb: Xlsx<_> = calamine::open_workbook_from_rs(std::io::Cursor::new(data.to_vec()))
		.map_err(|e| lformat!("не удалось открыть Excel-файл: {e}", "could not open the Excel file: {e}"))?;
	let text = wb.worksheet_range(TEXT_SHEET).or_else(|_| wb.worksheet_range(LEGACY_TEXT_SHEET)).map_err(|_| lformat!("нет листа «{TEXT_SHEET}»", "no '{TEXT_SHEET}' sheet"))?;
	let dat = wb.worksheet_range(DATA_SHEET).or_else(|_| wb.worksheet_range(LEGACY_DATA_SHEET)).map_err(|_| lformat!("нет скрытого листа «{DATA_SHEET}» — файл создан не этой программой или лист удалён", "no hidden '{DATA_SHEET}' sheet: the file was not created by this program or the sheet was deleted"))?;

	let mut texts: HashMap<String, (String, usize)> = HashMap::new();
	for (r, row) in text.rows().enumerate().skip(1) {
		let id = row.first().map(cell_to_string).unwrap_or_default().trim().to_string();
		if id.is_empty() { continue }
		let t = row.get(1).map(cell_to_string).unwrap_or_default().replace("\r\n", "\n").replace('\r', "\n");
		let t = crate::text::normalize(&t).into_owned();
		if let Err(e) = crate::text::encode(&t) {
			return Err(lformat!("лист «{TEXT_SHEET}», строка {} (ID {id}): {}", "sheet '{TEXT_SHEET}', line {} (ID {id}): {}", r + 1, e.message));
		}
		texts.insert(id, (t, r + 1));
	}

	let mut lines: Vec<String> = Vec::new();
	let mut first = true;
	for row in dat.rows() {
		if first {
			first = false;
			if row.first().map(cell_to_string).as_deref() != Some(MAGIC) {
				return Err(tr("служебный лист повреждён", "the service sheet is damaged").into());
			}
			continue;
		}
		let c = row.first().map(cell_to_string).unwrap_or_default();
		let cont = row.get(1).map(cell_to_string).as_deref() == Some("+");
		if cont {
			if let Some(l) = lines.last_mut() { l.push_str(&c) }
		} else {
			lines.push(c);
		}
	}

	let mut out = String::from(tr("; kiseki-tool table (из Excel)\n\n", "; kiseki-tool table (from Excel)\n\n"));
	for l in lines {
		if let Some(rest) = l.strip_prefix("@@ ") {
			let id = rest.split_whitespace().next().unwrap_or("").to_string();
			let (t, _) = texts.remove(&id).ok_or_else(|| lformat!("на листе «{TEXT_SHEET}» нет строки с ID {id} (строки удалять нельзя)", "the '{TEXT_SHEET}' sheet has no line with ID {id} (lines must not be deleted)"))?;
			out.push_str(&l);
			out.push('\n');
			model::push_text_lines(&mut out, &t);
		} else {
			out.push_str(&l);
			out.push('\n');
		}
	}
	if let Some((id, (_, r))) = texts.into_iter().next() {
		return Err(lformat!("лист «{TEXT_SHEET}», строка {r}: неизвестный ID {id} (добавлять строки нельзя)", "sheet '{TEXT_SHEET}', line {r}: unknown ID {id} (lines must not be added)"));
	}
	Ok(out)
}
