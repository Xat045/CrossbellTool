#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use std::collections::HashMap;

use serde_json::{json, Map, Value};

static SCHEMAS: &str = include_str!("kuro_schemas.json");
const AUTO: &str = "auto";

struct Sec {
	name: String,
	start: usize,
	len: usize,
	count: usize,
}

fn u32at(d: &[u8], o: usize) -> Result<usize, String> {
	d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize).ok_or_else(|| tr("файл обрезан", "the file is truncated").to_string())
}

fn sections(d: &[u8]) -> Result<Vec<Sec>, String> {
	if !d.starts_with(b"#TBL") {
		return Err(tr("это не таблица #TBL", "this is not a #TBL table").into());
	}
	let mut out = Vec::new();
	for i in 0..u32at(d, 4)? {
		let o = 8 + i * 80;
		let raw = d.get(o..o + 64).ok_or(tr("файл обрезан", "the file is truncated"))?;
		let end = raw.iter().position(|&b| b == 0).unwrap_or(64);
		let s = Sec { name: String::from_utf8_lossy(&raw[..end]).into_owned(), start: u32at(d, o + 68)?, len: u32at(d, o + 72)?, count: u32at(d, o + 76)? };
		if s.len.checked_mul(s.count).and_then(|n| n.checked_add(s.start)).is_none_or(|e| e > d.len()) {
			return Err(lformat!("раздел {} выходит за конец файла", "section {} goes past the end of the file", s.name));
		}
		out.push(s);
	}
	Ok(out)
}

fn schema_for(d: &[u8], s: &Sec, end: usize) -> Option<(String, Map<String, Value>)> {
	thread_local! {
		static DB: Value = serde_json::from_str(SCHEMAS).unwrap();
	}
	let known = DB.with(|db| {
		let list = db.get(&s.name).and_then(|v| v.as_array());
		let same: Vec<&Value> = list.into_iter().flatten().filter(|x| x["size"].as_u64() == Some(s.len as u64)).collect();
		let tag = super::game::current().schema_tag();
		let pick = same.iter().find(|x| x["game"].as_str().is_some_and(|g| g.starts_with(tag))).or(same.first());
		pick.and_then(|x| Some((x["game"].as_str().unwrap_or("").to_string(), x["schema"].as_object()?.clone())))
	});
	let fits = |m: &Map<String, Value>| {
		(0..s.count).all(|i| {
			let mut pos = s.start + i * s.len;
			m.values().all(|t| read(d, &mut pos, t).is_some())
		})
	};
	known.filter(|(_, m)| fits(m)).or_else(|| infer(d, s, end).map(|m| (AUTO.to_string(), m)))
}

fn data_end(secs: &[Sec]) -> usize {
	secs.iter().map(|s| s.start + s.len * s.count).max().unwrap_or(0)
}

fn is_text_ptr(d: &[u8], s: &Sec, off: usize, end: usize) -> bool {
	let mut any = false;
	for i in 0..s.count {
		let at = s.start + i * s.len + off;
		let p = u64::from_le_bytes(d[at..at + 8].try_into().unwrap());
		if p == 0 {
			continue;
		}
		let Ok(p) = usize::try_from(p) else { return false };
		if p < end || p >= d.len() {
			return false;
		}
		match cstr(d, p, false) {
			Some(t) if !t.chars().any(|c| c.is_control() && c != '\n') => any |= !t.is_empty(),
			_ => return false,
		}
	}
	any
}

fn is_float(d: &[u8], s: &Sec, off: usize) -> bool {
	let mut any = false;
	for i in 0..s.count {
		let at = s.start + i * s.len + off;
		let b = u32::from_le_bytes(d[at..at + 4].try_into().unwrap());
		if b == 0 {
			continue;
		}
		let f = f32::from_bits(b).abs();
		if !f.is_finite() || !(1e-4..=1e7).contains(&f) {
			return false;
		}
		any = true;
	}
	any
}

fn infer(d: &[u8], s: &Sec, end: usize) -> Option<Map<String, Value>> {
	if s.count == 0 || s.len == 0 {
		return None;
	}
	let mut m = Map::new();
	let mut off = 0;
	while off < s.len {
		let left = s.len - off;
		let (name, ty, n) = if off % 8 == 0 && left >= 8 && is_text_ptr(d, s, off, end) {
			("field", "toffset", 8)
		} else if off % 4 == 0 && left >= 4 {
			if is_float(d, s, off) { ("float", "float", 4) } else { ("int", "int", 4) }
		} else if off % 2 == 0 && left >= 2 {
			("short", "short", 2)
		} else {
			("byte", "ubyte", 1)
		};
		m.insert(format!("{name}{off}"), json!(ty));
		off += n;
	}
	Some(m)
}

fn cstr(d: &[u8], p: usize, latin: bool) -> Option<String> {
	let rest = d.get(p..)?;
	let b = &rest[..rest.iter().position(|&c| c == 0)?];
	if latin { Some(b.iter().map(|&c| c as char).collect()) } else { std::str::from_utf8(b).ok().map(str::to_string) }
}

fn int_size(t: &str) -> Option<(usize, bool)> {
	let (u, base) = t.strip_prefix('u').map_or((false, t), |r| (true, r));
	let n = match base {
		"byte" => 1,
		"short" => 2,
		"int" => 4,
		"long" => 8,
		_ => return None,
	};
	Some((n, !u))
}

fn array_elem(t: &str) -> Option<usize> {
	Some(t.strip_prefix('u')?.strip_suffix("array")?.parse::<usize>().ok()? / 8)
}

fn read_int(d: &[u8], o: usize, n: usize, signed: bool) -> Option<Value> {
	let b = d.get(o..o + n)?;
	let mut x = [0u8; 8];
	x[..n].copy_from_slice(b);
	let u = u64::from_le_bytes(x);
	if signed {
		let sh = 64 - n as u32 * 8;
		Some(json!(((u << sh) as i64) >> sh))
	} else {
		Some(json!(u))
	}
}

fn float_json(f: f32) -> Value {
	if f.is_finite() {
		format!("{f}").parse::<f64>().ok().and_then(serde_json::Number::from_f64).map_or(json!(0.0), Value::Number)
	} else {
		json!(format!("0x{:08X}", f.to_bits()))
	}
}

fn hex(b: &[u8]) -> String {
	b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")
}

fn read(d: &[u8], pos: &mut usize, ty: &Value) -> Option<Value> {
	if let Some(o) = ty.as_object() {
		let n = o.get("size")?.as_u64()?;
		let inner = o.get("schema")?.as_object()?;
		let mut out = Vec::new();
		for _ in 0..n {
			let mut m = Map::new();
			for (k, t) in inner {
				m.insert(k.clone(), read(d, pos, t)?);
			}
			out.push(Value::Object(m));
		}
		return Some(Value::Array(out));
	}
	let t = ty.as_str()?;
	if let Some((n, s)) = int_size(t) {
		let v = read_int(d, *pos, n, s)?;
		*pos += n;
		return Some(v);
	}
	if t == "float" {
		let v = f32::from_le_bytes(d.get(*pos..*pos + 4)?.try_into().ok()?);
		*pos += 4;
		return Some(float_json(v));
	}
	if let Some(enc) = t.strip_prefix("toffset") {
		let p = u64::from_le_bytes(d.get(*pos..*pos + 8)?.try_into().ok()?) as usize;
		*pos += 8;
		return Some(Value::String(if p == 0 { String::new() } else { cstr(d, p, !enc.is_empty())? }));
	}
	if let Some(n) = array_elem(t) {
		let p = u64::from_le_bytes(d.get(*pos..*pos + 8)?.try_into().ok()?) as usize;
		let c = u32at(d, *pos + 8).ok()?;
		*pos += 12;
		let mut out = Vec::new();
		for i in 0..c {
			out.push(read_int(d, p.checked_add(i.checked_mul(n)?)?, n, false)?);
		}
		return Some(Value::Array(out));
	}
	if let Some(n) = t.strip_prefix("data").and_then(|n| n.parse::<usize>().ok()) {
		let b = d.get(*pos..*pos + n)?;
		*pos += n;
		return Some(Value::String(hex(b)));
	}
	None
}

pub fn decompile(d: &[u8]) -> Result<Vec<u8>, String> {
	let secs = sections(d)?;
	let end = data_end(&secs);
	let mut headers = Vec::new();
	let mut data = Vec::new();
	for s in &secs {
		let parsed = schema_for(d, s, end).and_then(|(game, fields)| {
			let mut rows = Vec::with_capacity(s.count);
			for i in 0..s.count {
				let mut pos = s.start + i * s.len;
				let mut m = Map::new();
				for (k, t) in &fields {
					m.insert(k.clone(), read(d, &mut pos, t)?);
				}
				rows.push(Value::Object(m));
			}
			Some((game, rows))
		});
		let mut h = Map::new();
		h.insert("name".into(), json!(s.name));
		let rows = match parsed {
			Some((game, rows)) => {
				h.insert("schema".into(), json!(game));
				rows
			}
			None => (0..s.count).map(|i| json!({ "data": hex(&d[s.start + i * s.len..s.start + (i + 1) * s.len]) })).collect(),
		};
		headers.push(Value::Object(h));
		data.push(json!({ "name": s.name, "data": rows }));
	}
	let doc = json!({ "headers": headers, "data": data });
	let mut out = Vec::new();
	let fmt = serde_json::ser::PrettyFormatter::with_indent(b"\t");
	let mut ser = serde_json::Serializer::with_formatter(&mut out, fmt);
	serde::Serialize::serialize(&doc, &mut ser).map_err(|e| e.to_string())?;
	if compile(&out, d)? != d {
		return Err(tr("внутренняя ошибка: после обратной сборки таблица отличается от исходной", "internal error: the table differs from the original after building it back").into());
	}
	Ok(out)
}

struct Out {
	b: Vec<u8>,
	added: HashMap<String, u64>,
}

fn put_int(o: &mut Out, at: usize, n: usize, signed: bool, v: &Value, what: &str) -> Result<(), String> {
	let (fits, raw) = if signed {
		let x = v.as_i64().ok_or_else(|| lformat!("{what}: нужно целое число", "{what}: an integer is expected"))?;
		let sh = 64 - n as u32 * 8;
		(((x << sh) >> sh) == x, x as u64)
	} else {
		let x = v.as_u64().ok_or_else(|| lformat!("{what}: нужно целое неотрицательное число", "{what}: a non-negative integer is expected"))?;
		(n == 8 || x >> (n * 8) == 0, x)
	};
	if !fits {
		return Err(lformat!("{what}: значение {v} не помещается в поле ({} байт)", "{what}: the value {v} doesn't fit in the field ({} bytes)", n));
	}
	o.b[at..at + n].copy_from_slice(&raw.to_le_bytes()[..n]);
	Ok(())
}

fn write(o: &mut Out, orig: &[u8], pos: &mut usize, ty: &Value, v: &Value, what: &str) -> Result<(), String> {
	if let Some(t) = ty.as_object() {
		let n = t.get("size").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
		let inner = t.get("schema").and_then(|x| x.as_object()).ok_or(tr("повреждена схема", "the schema is corrupted"))?;
		let list = v.as_array().filter(|a| a.len() == n).ok_or_else(|| lformat!("{what}: нужен список из {n} элементов", "{what}: a list of {n} items is expected"))?;
		for (i, it) in list.iter().enumerate() {
			let m = it.as_object().ok_or_else(|| lformat!("{what}[{i}]: нужен объект", "{what}[{i}]: an object is expected"))?;
			for (k, ft) in inner {
				let x = m.get(k).ok_or_else(|| lformat!("{what}[{i}]: нет поля {k}", "{what}[{i}]: no field {k}"))?;
				write(o, orig, pos, ft, x, &format!("{what}[{i}].{k}"))?;
			}
		}
		return Ok(());
	}
	let t = ty.as_str().ok_or(tr("повреждена схема", "the schema is corrupted"))?;
	if let Some((n, s)) = int_size(t) {
		put_int(o, *pos, n, s, v, what)?;
		*pos += n;
		return Ok(());
	}
	if t == "float" {
		let bits = match v {
			Value::String(h) => u32::from_str_radix(h.trim_start_matches("0x"), 16).map_err(|_| lformat!("{what}: нужно число", "{what}: a number is expected"))?,
			_ => (v.as_f64().ok_or_else(|| lformat!("{what}: нужно число", "{what}: a number is expected"))? as f32).to_bits(),
		};
		let old = u32::from_le_bytes(orig[*pos..*pos + 4].try_into().unwrap());
		if float_json(f32::from_bits(old)) != *v {
			o.b[*pos..*pos + 4].copy_from_slice(&bits.to_le_bytes());
		}
		*pos += 4;
		return Ok(());
	}
	if let Some(enc) = t.strip_prefix("toffset") {
		let text = v.as_str().ok_or_else(|| lformat!("{what}: нужна строка", "{what}: a string is expected"))?;
		let p = u64::from_le_bytes(orig[*pos..*pos + 8].try_into().unwrap()) as usize;
		let latin = !enc.is_empty();
		let old = if p == 0 { String::new() } else { cstr(orig, p, latin).unwrap_or_default() };
		if old != text {
			if text.contains('\0') {
				return Err(lformat!("{what}: недопустимый символ", "{what}: invalid character"));
			}
			let np = match o.added.get(text) {
				Some(&x) => x,
				None => {
					let x = o.b.len() as u64;
					if latin {
						for c in text.chars() {
							o.b.push(u8::try_from(c as u32).map_err(|_| lformat!("{what}: символ «{c}» не поддерживается этим полем", "{what}: this field doesn't support the character '{c}'"))?);
						}
					} else {
						o.b.extend_from_slice(text.as_bytes());
					}
					o.b.push(0);
					o.added.insert(text.to_string(), x);
					x
				}
			};
			o.b[*pos..*pos + 8].copy_from_slice(&np.to_le_bytes());
		}
		*pos += 8;
		return Ok(());
	}
	if let Some(n) = array_elem(t) {
		let list = v.as_array().ok_or_else(|| lformat!("{what}: нужен список", "{what}: a list is expected"))?;
		let mut r = *pos;
		let old = read(orig, &mut r, ty);
		if old.as_ref() != Some(v) {
			while o.b.len() % n != 0 {
				o.b.push(0);
			}
			let np = o.b.len() as u64;
			for (i, x) in list.iter().enumerate() {
				let at = o.b.len();
				o.b.resize(at + n, 0);
				put_int(o, at, n, false, x, &format!("{what}[{i}]"))?;
			}
			o.b[*pos..*pos + 8].copy_from_slice(&np.to_le_bytes());
			o.b[*pos + 8..*pos + 12].copy_from_slice(&(list.len() as u32).to_le_bytes());
		}
		*pos += 12;
		return Ok(());
	}
	if let Some(n) = t.strip_prefix("data").and_then(|n| n.parse::<usize>().ok()) {
		let b = parse_hex(v.as_str().ok_or_else(|| lformat!("{what}: нужна строка hex", "{what}: a hex string is expected"))?, what)?;
		if b.len() != n {
			return Err(lformat!("{what}: нужно {n} байт, указано {}", "{what}: {n} bytes are needed, {} given", b.len()));
		}
		o.b[*pos..*pos + n].copy_from_slice(&b);
		*pos += n;
		return Ok(());
	}
	Err(lformat!("{what}: неизвестный тип поля {t}", "{what}: unknown field type {t}"))
}

fn parse_hex(s: &str, what: &str) -> Result<Vec<u8>, String> {
	s.split_whitespace().map(|x| u8::from_str_radix(x, 16).map_err(|_| lformat!("{what}: неверные данные «{x}»", "{what}: invalid data '{x}'"))).collect()
}

pub fn compile(src: &[u8], orig: &[u8]) -> Result<Vec<u8>, String> {
	let text = std::str::from_utf8(src).map_err(|_| tr("файл не в кодировке UTF-8", "the file is not in UTF-8").to_string())?;
	let doc: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| lformat!("ошибка JSON: {e}", "JSON error: {e}"))?;
	let secs = sections(orig)?;
	let end = data_end(&secs);
	let data = doc.get("data").and_then(|v| v.as_array()).ok_or(tr("нет раздела data", "no data section"))?;
	if data.len() != secs.len() {
		return Err(lformat!("разделов {}, а в таблице игры {}: разделы менять нельзя", "{} sections, but the game table has {}: sections can't be changed", data.len(), secs.len()));
	}
	let mut o = Out { b: orig.to_vec(), added: HashMap::new() };
	for (s, block) in secs.iter().zip(data) {
		if block.get("name").and_then(|v| v.as_str()) != Some(s.name.as_str()) {
			return Err(lformat!("раздел {} идёт не на своём месте", "section {} is out of place", s.name));
		}
		let rows = block.get("data").and_then(|v| v.as_array()).ok_or_else(|| lformat!("{}: нет записей", "{}: no records", s.name))?;
		if rows.len() != s.count {
			return Err(lformat!("{}: записей {}, а в таблице игры {}: добавлять и удалять записи нельзя", "{}: {} records, but the game table has {}: records can't be added or removed", s.name, rows.len(), s.count));
		}
		let sch = schema_for(orig, s, end);
		for (i, row) in rows.iter().enumerate() {
			let at = s.start + i * s.len;
			let what = format!("{}#{i}", s.name);
			let m = row.as_object().ok_or_else(|| lformat!("{what}: нужен объект", "{what}: an object is expected"))?;
			let hex_row = m.len() == 1 && m.get("data").is_some_and(|v| v.is_string()) && !sch.as_ref().is_some_and(|x| x.1.contains_key("data"));
			if hex_row {
				let b = parse_hex(m["data"].as_str().unwrap_or(""), &what)?;
				if b.len() != s.len {
					return Err(lformat!("{what}: нужно {} байт, указано {}", "{what}: {} bytes are needed, {} given", s.len, b.len()));
				}
				o.b[at..at + s.len].copy_from_slice(&b);
				continue;
			}
			let (_, fields) = sch.as_ref().ok_or_else(|| lformat!("{what}: нет схемы для этого раздела, нужно поле data", "{what}: this section has no schema, the data field is needed"))?;
			let mut pos = at;
			for (k, t) in fields {
				let v = m.get(k).ok_or_else(|| lformat!("{what}: нет поля {k}", "{what}: no field {k}"))?;
				write(&mut o, orig, &mut pos, t, v, &format!("{what}.{k}"))?;
			}
			if let Some(k) = m.keys().find(|k| !fields.contains_key(*k)) {
				return Err(lformat!("{what}: неизвестное поле {k}", "{what}: unknown field {k}"));
			}
		}
	}
	Ok(o.b)
}

pub fn texts(doc: &Value) -> Vec<(String, String)> {
	let mut out = Vec::new();
	for block in doc.get("data").and_then(|v| v.as_array()).into_iter().flatten() {
		let Some(name) = block.get("name").and_then(|v| v.as_str()) else { continue };
		let names = super::tbl::text_field_names(name);
		let rows = block.get("data").and_then(|v| v.as_array()).map_or(&[][..], |a| a.as_slice());
		let found = |k: &str| {
			k.strip_prefix("field").is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
				&& super::tbl::known_text_section(name)
				&& rows.iter().filter_map(|r| r[k].as_str()).any(|s| !s.is_empty() && !super::tbl::identifier(s) && !s.contains('/') && (s.contains(' ') || !s.is_ascii()))
		};
		for (i, row) in rows.iter().enumerate() {
			let Some(m) = row.as_object() else { continue };
			for (k, v) in m {
				if let Some(t) = v.as_str().filter(|t| !t.is_empty() && (names.iter().any(|n| n == k) || found(k)) && !super::tbl::identifier(t) && !t.chars().any(|c| c.is_control() && c != '\n')) {
					out.push((format!("{name}#{i}.{k}"), t.to_string()));
				}
			}
		}
	}
	let tech = super::tbl::technical_set(&out);
	out.retain(|(id, _)| !tech.contains(&super::tbl::field_key(id)));
	out
}
