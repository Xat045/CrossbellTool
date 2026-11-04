#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
pub mod model;
pub mod formats;
pub mod xlsx;

use model::{Disc, Doc};

pub const MAGIC: &str = "; kiseki-tool table";

pub fn is_supported(stem: &str) -> bool {
	formats::find(stem).is_some()
}

pub fn decompile(data: &[u8], stem: &str, file_name: &str) -> Result<(String, usize), String> {
	let fmt = formats::find(stem).ok_or_else(|| tr("формат этой таблицы пока не поддерживается", "this table's format is not supported yet").to_string())?;
	let mut d = Disc::new(data);
	(fmt.rule)(&mut d)?;
	let doc = d.finish()?;
	let header = lformat!("{MAGIC} | файл: {file_name} | формат: {}\n\
		 ; Редактируйте текст под строками вида «@@ s12». Перевод строки в тексте = перевод строки в игре.\n\
		 ; {{xNN}} — служебный байт (цвет, пауза и т.п.), {{{{ — символ «{{».\n\
		 ; Строки «@@@ dN: ...» — служебные данные с указателями, их лучше не трогать.\n\
		 ; Метки (s12, d3) и их порядок менять нельзя.\n\n", "{MAGIC} | file: {file_name} | format: {}\n; Edit the text under lines like '@@ s12'. A line break in the text is a line break in the game.\n; {{xNN}} is a service byte (color, pause and so on), {{{{ is the '{{' character.\n; Lines '@@@ dN: ...' are service data with pointers, better leave them alone.\n; Labels (s12, d3) and their order must not change.\n\n",
		fmt.name
	);
	let text = doc.to_text(&header);
	let back = Doc::from_text(&text).map_err(|e| lformat!("внутренняя ошибка проверки: {e}", "internal check error: {e}"))?;
	let bin = back.build().map_err(|e| lformat!("внутренняя ошибка проверки: {e}", "internal check error: {e}"))?;
	if bin != data {
		return Err(tr("внутренняя ошибка: после обратной сборки файл отличается от исходного", "internal error: after building back the file differs from the source").into());
	}
	Ok((text, doc.string_count()))
}

pub fn compile(src: &str) -> Result<Vec<u8>, String> {
	let src = crate::text::normalize(src);
	let doc = Doc::from_text(&src)?;
	doc.build()
}

pub fn looks_like_table_text(src: &str) -> bool {
	src.trim_start_matches('\u{FEFF}').starts_with(MAGIC)
}

pub fn decompile_xlsx(data: &[u8], stem: &str, file_name: &str) -> Result<(Vec<u8>, usize), String> {
	let fmt = formats::find(stem).ok_or_else(|| tr("формат этой таблицы пока не поддерживается", "this table's format is not supported yet").to_string())?;
	let mut d = Disc::new(data);
	(fmt.rule)(&mut d)?;
	let doc = d.finish()?;
	let book = xlsx::write(&doc, file_name, fmt.name)?;
	let back = compile_xlsx(&book).map_err(|e| lformat!("внутренняя ошибка проверки: {e}", "internal check error: {e}"))?;
	if back != data {
		return Err(tr("внутренняя ошибка: после обратной сборки файл отличается от исходного", "internal error: after building back the file differs from the source").into());
	}
	Ok((book, doc.string_count()))
}

pub fn compile_xlsx(book: &[u8]) -> Result<Vec<u8>, String> {
	let text = xlsx::to_table_text(book)?;
	let doc = Doc::from_text(&text)?;
	doc.build()
}

pub fn compile_xlsx_checked(book: &[u8], orig: &[u8], stem: &str) -> Result<(Vec<u8>, Vec<String>), String> {
	use model::Item;
	let text = xlsx::to_table_text(book)?;
	let mut doc = Doc::from_text(&text)?;
	let od = formats::find(stem).and_then(|fmt| {
		let mut d = Disc::new(orig);
		(fmt.rule)(&mut d).ok()?;
		d.finish().ok()
	});
	let mut fixed = Vec::new();
	if let Some(od) = od.filter(|od| od.items.len() == doc.items.len()) {
		for i in 0..od.items.len() {
			let restore = match (&od.items[i], &doc.items[i]) {
				(Item::Str { .. }, Item::Str { .. }) => {
					let ot = od.item_text(i).unwrap_or_default();
					let r = crate::kai::compare::identifier(ot.trim()) && doc.item_text(i).as_deref() != Some(ot.as_str());
					if r {
						fixed.push(format!("«{}» (s{i})", ot.trim()));
					}
					r
				}
				_ => false,
			};
			if restore {
				doc.items[i] = od.items[i].clone();
			}
		}
	}
	Ok((doc.build()?, fixed))
}

fn keyed(doc: &Doc) -> Vec<(String, String, String)> {
	use model::{Item, Tok};
	let mut refkey: Vec<Option<String>> = vec![None; doc.items.len()];
	let mut di = 0usize;
	for it in &doc.items {
		if let Item::Data(toks) = it {
			let mut pi = 0usize;
			for t in toks {
				if let Tok::Ptr { target, .. } = t {
					if let Some(slot) = refkey.get_mut(*target) {
						if slot.is_none() {
							*slot = Some(format!("d{di}.{pi}"));
						}
					}
					pi += 1;
				}
			}
			di += 1;
		}
	}
	let mut out = Vec::new();
	let mut k = 0usize;
	for i in 0..doc.items.len() {
		if let Some(t) = doc.item_text(i) {
			let key = refkey[i].clone().unwrap_or_else(|| format!("#{k}"));
			out.push((format!("s{i}"), key, t));
			k += 1;
		}
	}
	out
}

pub fn texts(data: &[u8], stem: &str) -> Result<Vec<(String, String, String)>, String> {
	let fmt = formats::find(stem).ok_or_else(|| tr("формат этой таблицы пока не поддерживается", "this table's format is not supported yet").to_string())?;
	let mut d = Disc::new(data);
	(fmt.rule)(&mut d)?;
	let doc = d.finish()?;
	Ok(keyed(&doc))
}

pub fn texts_xlsx(book: &[u8]) -> Result<Vec<(String, String, String)>, String> {
	let text = xlsx::to_table_text(book)?;
	let doc = Doc::from_text(&text)?;
	Ok(keyed(&doc))
}
