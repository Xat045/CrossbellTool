#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use crate::text;

pub type Res<T> = Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
	Str,
	Data(usize),
	Open,
}

#[derive(Clone)]
pub struct Disc<'a> {
	pub d: &'a [u8],
	items: BTreeMap<usize, Kind>,
	ptrs: BTreeMap<usize, (u8, usize)>,
}

impl<'a> Disc<'a> {
	pub fn new(d: &'a [u8]) -> Self {
		Disc { d, items: BTreeMap::new(), ptrs: BTreeMap::new() }
	}

	pub fn len(&self) -> usize { self.d.len() }

	pub fn u8(&self, o: usize) -> Res<usize> {
		self.d.get(o).map(|&v| v as usize).ok_or_else(|| lformat!("чтение за концом файла (0x{o:X})", "reading past the end of the file (0x{o:X})"))
	}
	pub fn u16(&self, o: usize) -> Res<usize> {
		self.d.get(o..o + 2)
			.map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
			.ok_or_else(|| lformat!("чтение за концом файла (0x{o:X})", "reading past the end of the file (0x{o:X})"))
	}
	pub fn u32(&self, o: usize) -> Res<usize> {
		self.d.get(o..o + 4)
			.map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
			.ok_or_else(|| lformat!("чтение за концом файла (0x{o:X})", "reading past the end of the file (0x{o:X})"))
	}
	pub fn bytes(&self, a: usize, b: usize) -> Res<&'a [u8]> {
		self.d.get(a..b).ok_or_else(|| lformat!("чтение за концом файла (0x{a:X}..0x{b:X})", "reading past the end of the file (0x{a:X}..0x{b:X})"))
	}

	fn add(&mut self, o: usize, k: Kind) -> Res<()> {
		if o > self.d.len() {
			return Err(lformat!("элемент за концом файла (0x{o:X})", "element past the end of the file (0x{o:X})"));
		}
		match self.items.get(&o) {
			None => { self.items.insert(o, k); Ok(()) }
			Some(Kind::Open) => { self.items.insert(o, k); Ok(()) }
			Some(&old) if old == k || k == Kind::Open => Ok(()),
			Some(old) => Err(lformat!("конфликт структуры по адресу 0x{o:X}: {old:?} и {k:?}", "structure conflict at address 0x{o:X}: {old:?} and {k:?}")),
		}
	}

	pub fn string(&mut self, o: usize) -> Res<usize> {
		if o >= self.d.len() {
			return Err(lformat!("строка за концом файла (0x{o:X})", "string past the end of the file (0x{o:X})"));
		}
		if !self.d[o..].contains(&0) {
			return Err(lformat!("строка без завершающего нуля (0x{o:X})", "string without a terminating zero (0x{o:X})"));
		}
		self.add(o, Kind::Str)?;
		Ok(o)
	}

	pub fn data(&mut self, o: usize, n: usize) -> Res<usize> {
		if n == 0 { return Ok(o) }
		if o.checked_add(n).is_none_or(|e| e > self.d.len()) {
			return Err(lformat!("блок данных за концом файла (0x{o:X}+{n})", "data block past the end of the file (0x{o:X}+{n})"));
		}
		self.add(o, Kind::Data(n))?;
		Ok(o)
	}

	pub fn data_open(&mut self, o: usize) -> Res<usize> {
		if !self.items.contains_key(&o) {
			self.add(o, Kind::Open)?;
		}
		Ok(o)
	}

	pub fn p16(&mut self, slot: usize) -> Res<usize> {
		let t = self.u16(slot)?;
		self.ptrs.insert(slot, (2, t));
		Ok(t)
	}

	pub fn p32(&mut self, slot: usize) -> Res<usize> {
		let t = self.u32(slot)?;
		self.ptrs.insert(slot, (4, t));
		Ok(t)
	}

	pub fn str16(&mut self, slot: usize) -> Res<Option<usize>> {
		let t = self.u16(slot)?;
		if t == 0 { return Ok(None) }
		self.p16(slot)?;
		self.string(t).map(Some)
	}

	pub fn ptr_table(&mut self, start: usize, limit: usize) -> Res<Vec<usize>> {
		let mut p = start;
		let mut m = limit.min(self.d.len());
		let mut out = Vec::new();
		while p + 2 <= m {
			let v = self.u16(p)?;
			if v != 0 {
				if v <= start {
					return Err(lformat!("указатель 0x{v:X} в таблице 0x{start:X} направлен назад", "pointer 0x{v:X} in table 0x{start:X} points backwards"));
				}
				self.p16(p)?;
				out.push(v);
				m = m.min(v);
			}
			p += 2;
		}
		self.data(start, p - start)?;
		Ok(out)
	}

	pub fn strings_until(&mut self, mut o: usize, end: usize) -> Res<()> {
		while o < end {
			self.string(o)?;
			o += self.d[o..].iter().position(|&b| b == 0).unwrap() + 1;
		}
		if o != end {
			return Err(lformat!("строки выходят за границу блока 0x{end:X}", "strings cross the block boundary 0x{end:X}"));
		}
		Ok(())
	}

	pub fn finish(self) -> Res<Doc> {
		let d = self.d;
		let mut items = self.items;
		let keys: Vec<usize> = items.keys().copied().collect();
		for (i, &o) in keys.iter().enumerate() {
			if items[&o] == Kind::Open {
				let next = keys.get(i + 1).copied().unwrap_or(d.len());
				items.insert(o, Kind::Data(next - o));
			}
		}

		let mut segs: Vec<(usize, usize, bool)> = Vec::new();
		let mut pos = 0usize;
		for (&o, &k) in &items {
			if o < pos {
				return Err(lformat!("элементы перекрываются по адресу 0x{o:X}", "elements overlap at address 0x{o:X}"));
			}
			if o > pos {
				segs.push((pos, o, false));
			}
			let end = match k {
				Kind::Str => o + d[o..].iter().position(|&b| b == 0).unwrap() + 1,
				Kind::Data(n) => o + n,
				Kind::Open => unreachable!(),
			};
			segs.push((o, end, k == Kind::Str));
			pos = end;
		}
		if pos < d.len() {
			segs.push((pos, d.len(), false));
		}

		let starts: BTreeSet<usize> = items.keys().copied().collect();
		let mut out: Vec<Item> = Vec::new();
		let mut index_of: HashMap<usize, usize> = HashMap::new();
		let mut item_start: Vec<usize> = Vec::new();
		for &(a, b, is_str) in &segs {
			let is_gap = !starts.contains(&a);
			if is_gap {
				match out.last_mut() {
					Some(Item::Str { tail, .. }) => { tail.extend_from_slice(&d[a..b]); continue }
					Some(Item::Data(toks)) => { toks.push(Tok::Bytes(d[a..b].to_vec())); continue }
					None => {}
				}
			}
			index_of.insert(a, out.len());
			item_start.push(a);
			if is_str {
				out.push(Item::Str { bytes: d[a..b - 1].to_vec(), tail: Vec::new() });
			} else {
				out.push(Item::Data(vec![Tok::Bytes(d[a..b].to_vec())]));
			}
		}

		for (&slot, &(w, target)) in &self.ptrs {
			let Some((&istart, _)) = items.range(..=slot).next_back().map(|(a, b)| (a, b)) else {
				return Err(lformat!("указатель 0x{slot:X} вне блоков данных", "pointer 0x{slot:X} is outside the data blocks"));
			};
			let idx = index_of[&istart];
			let Some(&tidx) = index_of.get(&target) else {
				return Err(lformat!("указатель в 0x{slot:X} ведёт в 0x{target:X}, где нет начала элемента", "the pointer at 0x{slot:X} leads to 0x{target:X}, where no element starts"));
			};
			let Item::Data(toks) = &mut out[idx] else {
				return Err(lformat!("указатель 0x{slot:X} внутри строки", "pointer 0x{slot:X} is inside a string"));
			};
			let rel = slot - istart;
			split_ptr(toks, rel, w, tidx)?;
		}
		let _ = item_start;
		Ok(Doc { items: out })
	}
}

fn split_ptr(toks: &mut Vec<Tok>, rel: usize, w: u8, target: usize) -> Res<()> {
	let mut pos = 0;
	for i in 0..toks.len() {
		let len = toks[i].len();
		if rel >= pos && rel < pos + len {
			let Tok::Bytes(b) = &toks[i] else {
				return Err(tr("два указателя в одном месте", "two pointers in one place").into());
			};
			let off = rel - pos;
			if off + w as usize > b.len() {
				return Err(tr("указатель пересекает границу блока", "the pointer crosses a block boundary").into());
			}
			let before = b[..off].to_vec();
			let after = b[off + w as usize..].to_vec();
			let mut rep = Vec::new();
			if !before.is_empty() { rep.push(Tok::Bytes(before)) }
			rep.push(Tok::Ptr { w, target });
			if !after.is_empty() { rep.push(Tok::Bytes(after)) }
			toks.splice(i..=i, rep);
			return Ok(());
		}
		pos += len;
	}
	Err(tr("указатель за концом блока", "the pointer is past the end of the block").into())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
	Bytes(Vec<u8>),
	Ptr { w: u8, target: usize },
}

impl Tok {
	fn len(&self) -> usize {
		match self {
			Tok::Bytes(b) => b.len(),
			Tok::Ptr { w, .. } => *w as usize,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
	Data(Vec<Tok>),
	Str { bytes: Vec<u8>, tail: Vec<u8> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doc {
	pub items: Vec<Item>,
}

impl Doc {
	pub fn string_count(&self) -> usize {
		self.items.iter().filter(|i| matches!(i, Item::Str { .. })).count()
	}

	pub fn build(&self) -> Res<Vec<u8>> {
		let mut offs = Vec::with_capacity(self.items.len());
		let mut pos = 0usize;
		for it in &self.items {
			offs.push(pos);
			pos += match it {
				Item::Data(t) => t.iter().map(Tok::len).sum(),
				Item::Str { bytes, tail } => bytes.len() + 1 + tail.len(),
			};
		}
		let mut out = Vec::with_capacity(pos);
		for it in &self.items {
			match it {
				Item::Data(toks) => {
					for t in toks {
						match t {
							Tok::Bytes(b) => out.extend_from_slice(b),
							Tok::Ptr { w: 2, target } => {
								let v = offs[*target];
								if v > 0xFFFF {
									return Err(lformat!("файл стал слишком большим: адрес 0x{v:X} не помещается в 16-битный указатель (лимит 65535 байт). Сократите текст.", "the file has grown too large: address 0x{v:X} does not fit a 16-bit pointer (limit 65535 bytes). Shorten the text."
									));
								}
								out.extend_from_slice(&(v as u16).to_le_bytes());
							}
							Tok::Ptr { target, .. } => {
								out.extend_from_slice(&(offs[*target] as u32).to_le_bytes());
							}
						}
					}
				}
				Item::Str { bytes, tail } => {
					out.extend_from_slice(bytes);
					out.push(0);
					out.extend_from_slice(tail);
				}
			}
		}
		Ok(out)
	}

	pub fn item_header(&self, i: usize) -> String {
		let label = |i: usize| match &self.items[i] {
			Item::Data(_) => format!("d{i}"),
			Item::Str { .. } => format!("s{i}"),
		};
		let mut s = String::new();
		match &self.items[i] {
			Item::Data(toks) => {
				s.push_str(&format!("@@@ d{i}:"));
				for t in toks {
					match t {
						Tok::Bytes(b) => {
							for chunk in b.chunks(16) {
								s.push(' ');
								for x in chunk { s.push_str(&format!("{x:02x}")) }
							}
						}
						Tok::Ptr { w, target } => {
							s.push_str(&format!(" p{}:{}", *w as usize * 8, label(*target)));
						}
					}
				}
			}
			Item::Str { tail, .. } => {
				s.push_str(&format!("@@ s{i}"));
				if !tail.is_empty() {
					s.push_str(" +");
					for x in tail { s.push_str(&format!("{x:02x}")) }
				}
			}
		}
		s
	}

	pub fn item_text(&self, i: usize) -> Option<String> {
		match &self.items[i] {
			Item::Str { bytes, .. } => Some(text::decode(bytes)),
			_ => None,
		}
	}

	pub fn to_text(&self, header: &str) -> String {
		let mut s = String::new();
		s.push_str(header);
		for i in 0..self.items.len() {
			s.push_str(&self.item_header(i));
			s.push('\n');
			if let Some(t) = self.item_text(i) {
				push_text_lines(&mut s, &t);
			}
		}
		s
	}

	pub fn from_text(src: &str) -> Res<Doc> {
		let src = src.strip_prefix('\u{FEFF}').unwrap_or(src);
		let lines: Vec<&str> = src.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();

		enum Raw { Data(Vec<RawTok>), Str { tail: Vec<u8>, text: Vec<String>, line: usize } }
		enum RawTok { Bytes(Vec<u8>), Ptr(u8, String, usize) }

		let mut labels: HashMap<String, usize> = HashMap::new();
		let mut raws: Vec<Raw> = Vec::new();
		let mut started = false;

		for (ln, &line) in lines.iter().enumerate() {
			let lnum = ln + 1;
			if let Some(rest) = line.strip_prefix("@@@ ") {
				started = true;
				let (lab, body) = rest.split_once(':').ok_or_else(|| lformat!("строка {lnum}: ожидалось '@@@ dN: ...'", "line {lnum}: expected '@@@ dN: ...'"))?;
				let lab = lab.trim().to_string();
				if labels.insert(lab.clone(), raws.len()).is_some() {
					return Err(lformat!("строка {lnum}: метка {lab} повторяется", "line {lnum}: label {lab} is repeated"));
				}
				let mut toks = Vec::new();
				for w in body.split_whitespace() {
					if let Some(p) = w.strip_prefix("p16:") {
						toks.push(RawTok::Ptr(2, p.to_string(), lnum));
					} else if let Some(p) = w.strip_prefix("p32:") {
						toks.push(RawTok::Ptr(4, p.to_string(), lnum));
					} else {
						toks.push(RawTok::Bytes(parse_hex(w).ok_or_else(|| lformat!("строка {lnum}: неверные данные '{w}'", "line {lnum}: invalid data '{w}'"))?));
					}
				}
				raws.push(Raw::Data(toks));
			} else if line.starts_with("@@ ") || line == "@@" {
				started = true;
				let rest = line[2..].trim();
				let (lab, tail) = match rest.split_once(' ') {
					Some((l, t)) => (l, t.trim()),
					None => (rest, ""),
				};
				let tail = match tail.strip_prefix('+') {
					Some(h) => parse_hex(h).ok_or_else(|| lformat!("строка {lnum}: неверный хвост '{tail}'", "line {lnum}: invalid tail '{tail}'"))?,
					None if tail.is_empty() => Vec::new(),
					None => return Err(lformat!("строка {lnum}: лишний текст в заголовке строки: '{tail}'", "line {lnum}: extra text in the string header: '{tail}'")),
				};
				if labels.insert(lab.to_string(), raws.len()).is_some() {
					return Err(lformat!("строка {lnum}: метка {lab} повторяется", "line {lnum}: label {lab} is repeated"));
				}
				raws.push(Raw::Str { tail, text: Vec::new(), line: lnum });
			} else if !started {
				continue;
			} else {
				match raws.last_mut() {
					Some(Raw::Str { text, .. }) => text.push(line.to_string()),
					Some(Raw::Data(_)) => {
						if !line.trim().is_empty() {
							return Err(lformat!("строка {lnum}: текст после блока данных без заголовка '@@ sN'", "line {lnum}: text after a data block without an '@@ sN' header"));
						}
					}
					None => {}
				}
			}
		}

		if let Some(Raw::Str { text, .. }) = raws.last_mut() {
			if text.last().map(|s| s.is_empty()) == Some(true) {
				text.pop();
			}
		}

		let mut items = Vec::with_capacity(raws.len());
		for r in raws {
			match r {
				Raw::Data(toks) => {
					let mut out = Vec::new();
					for t in toks {
						match t {
							RawTok::Bytes(b) => out.push(Tok::Bytes(b)),
							RawTok::Ptr(w, l, lnum) => {
								let target = *labels.get(&l).ok_or_else(|| lformat!("строка {lnum}: нет метки {l}", "line {lnum}: label {l} is missing"))?;
								out.push(Tok::Ptr { w, target });
							}
						}
					}
					items.push(Item::Data(out));
				}
				Raw::Str { tail, text, line } => {
					let mut t = text.join("\n");
					if text.is_empty() { t.clear() }
					let bytes = text::encode(&t).map_err(|e| {
						let before: String = t.chars().take(e.char_index).collect();
						let extra = before.matches('\n').count();
						lformat!("строка {}: {}", "line {}: {}", line + 1 + extra, e.message)
					})?;
					if bytes.contains(&0) {
						return Err(lformat!("строка {}: символ {{x00}} внутри строки недопустим", "line {}: the {{x00}} character is not allowed inside a string", line + 1));
					}
					items.push(Item::Str { bytes, tail });
				}
			}
		}
		Ok(Doc { items })
	}
}

fn parse_hex(s: &str) -> Option<Vec<u8>> {
	if s.len() % 2 != 0 { return None }
	(0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

pub fn push_text_lines(s: &mut String, t: &str) {
	for line in t.split('\n') {
		if line.starts_with("@@") {
			s.push_str("{x40}");
			s.push_str(&line[1..]);
		} else {
			s.push_str(line);
		}
		s.push('\n');
	}
}
