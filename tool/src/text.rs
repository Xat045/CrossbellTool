#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
pub fn normalize(src: &str) -> std::borrow::Cow<'_, str> {
	if src.chars().any(|c| matches!(c, '\u{2014}' | '\u{00A0}')) {
		std::borrow::Cow::Owned(src.replace('\u{2014}', "\u{2015}").replace('\u{00A0}', " "))
	} else {
		std::borrow::Cow::Borrowed(src)
	}
}

pub fn decode(bytes: &[u8]) -> String {
	let mut out = String::with_capacity(bytes.len());
	let mut i = 0;
	while i < bytes.len() {
		let b = bytes[i];
		if b == 0x01 {
			out.push('\n');
			i += 1;
			continue;
		}
		if b < 0x20 || b == 0x7F {
			out.push_str(&format!("{{x{b:02X}}}"));
			i += 1;
			continue;
		}
		if b == b'{' {
			out.push_str("{{");
			i += 1;
			continue;
		}
		match cp932::decode_char(&bytes[i..]) {
			Some((ch, len)) if encode_char(ch).as_deref() == Some(&bytes[i..i + len]) => {
				out.push(ch);
				i += len;
			}
			_ => {
				out.push_str(&format!("{{x{b:02X}}}"));
				i += 1;
			}
		}
	}
	out
}

fn encode_char(ch: char) -> Option<Vec<u8>> {
	let mut buf = [0u8; 4];
	cp932::encode(ch.encode_utf8(&mut buf)).ok()
}

#[derive(Debug)]
pub struct EncodeError {
	pub char_index: usize,
	pub message: String,
}

pub fn encode(text: &str) -> Result<Vec<u8>, EncodeError> {
	let chars: Vec<char> = text.chars().collect();
	let mut out = Vec::with_capacity(text.len());
	let mut i = 0;
	while i < chars.len() {
		let c = chars[i];
		if c == '\n' {
			out.push(0x01);
			i += 1;
			continue;
		}
		if c == '\r' {
			i += 1;
			continue;
		}
		if c == '{' {
			if chars.get(i + 1) == Some(&'{') {
				out.push(b'{');
				i += 2;
				continue;
			}
			let esc: String = chars[i..chars.len().min(i + 5)].iter().collect();
			let bytes = esc.as_bytes();
			if esc.len() == 5 && (bytes[1] == b'x' || bytes[1] == b'X') && bytes[4] == b'}' {
				if let Ok(v) = u8::from_str_radix(&esc[2..4], 16) {
					out.push(v);
					i += 5;
					continue;
				}
			}
			return Err(EncodeError {
				char_index: i,
				message: tr("одиночная '{' — напишите '{{' для символа '{' или {xNN} для байта", "a single '{': write '{{' for the '{' character or {xNN} for a byte").into(),
			});
		}
		match encode_char(c) {
			Some(b) => out.extend(b),
			None => {
				return Err(EncodeError {
					char_index: i,
					message: lformat!("символ '{c}' (U+{:04X}) нельзя записать в кодировке игры (Shift-JIS)", "the character '{c}' (U+{:04X}) can't be written in the game's encoding (Shift-JIS)", c as u32),
				});
			}
		}
		i += 1;
	}
	Ok(out)
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn roundtrip() {
		let samples: &[&[u8]] = &[
			b"Hello\x01World\x07\x09{x}",
			b"\x84\x40\x84\x71 test",
			b"\x81\x40\x81\x9a",
			b"\x80\xa0\xff\xfe",
		];
		for s in samples {
			let t = decode(s);
			assert_eq!(&encode(&t).unwrap()[..], *s, "{t:?}");
		}
	}
}
