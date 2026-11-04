#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use block_compression::{BC7Settings, CompressionVariant};
use image::RgbaImage;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Fmt {
	Bc(u8),
	Rgba,
	Bgra,
}

struct Info {
	w: u32,
	h: u32,
	mips: u32,
	fmt: Fmt,
	data_off: usize,
}

fn u32at(d: &[u8], o: usize) -> u32 {
	d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).unwrap_or(0)
}

fn info(d: &[u8]) -> Result<Info, String> {
	if !d.starts_with(b"DDS ") || d.len() < 128 {
		return Err(tr("это не DDS", "this is not a DDS").into());
	}
	let h = u32at(d, 12);
	let w = u32at(d, 16);
	let mips = u32at(d, 28).max(1);
	if w == 0 || h == 0 || w > 16384 || h > 16384 {
		return Err(lformat!("неверный размер текстуры {w}x{h}", "invalid texture size {w}x{h}"));
	}
	let caps2 = u32at(d, 112);
	if caps2 & 0x200 != 0 {
		return Err(tr("кубические текстуры не поддерживаются", "cube textures are not supported").into());
	}
	let pf_flags = u32at(d, 80);
	let four = &d[84..88];
	let (fmt, off) = match four {
		b"DXT1" => (Fmt::Bc(1), 128),
		b"DXT3" => (Fmt::Bc(2), 128),
		b"DXT5" => (Fmt::Bc(3), 128),
		b"DX10" => {
			let dxgi = u32at(d, 128);
			if u32at(d, 140) > 1 {
				return Err(tr("массивы текстур не поддерживаются", "texture arrays are not supported").into());
			}
			let f = match dxgi {
				70..=72 => Fmt::Bc(1),
				73..=75 => Fmt::Bc(2),
				76..=78 => Fmt::Bc(3),
				97..=99 => Fmt::Bc(7),
				27..=29 => Fmt::Rgba,
				87 | 88 | 90 | 91 => Fmt::Bgra,
				x => return Err(lformat!("формат DXGI {x} не поддерживается", "DXGI format {x} is not supported")),
			};
			(f, 148)
		}
		_ if pf_flags & 0x40 != 0 && u32at(d, 88) == 32 => {
			if u32at(d, 92) == 0x00FF0000 { (Fmt::Bgra, 128) } else { (Fmt::Rgba, 128) }
		}
		_ => return Err(lformat!("формат {:?} не поддерживается", "format {:?} is not supported", String::from_utf8_lossy(four))),
	};
	Ok(Info { w, h, mips, fmt, data_off: off })
}

fn variant(n: u8) -> CompressionVariant {
	match n {
		1 => CompressionVariant::BC1,
		2 => CompressionVariant::BC2,
		3 => CompressionVariant::BC3,
		_ => CompressionVariant::BC7(BC7Settings::alpha_fast()),
	}
}

fn level_size(fmt: Fmt, w: u32, h: u32) -> usize {
	match fmt {
		Fmt::Bc(n) => variant(n).blocks_byte_size(w.max(1), h.max(1)),
		_ => (w.max(1) * h.max(1) * 4) as usize,
	}
}

pub fn supported(d: &[u8]) -> bool {
	info(d).is_ok()
}

pub struct Decoded {
	pub png: Vec<u8>,
}

pub fn decompile(d: &[u8]) -> Result<Decoded, String> {
	let i = info(d)?;
	let size = level_size(i.fmt, i.w, i.h);
	let data = d.get(i.data_off..i.data_off + size).ok_or(tr("DDS обрезан", "the DDS is truncated"))?;
	let mut rgba = vec![0u8; (i.w * i.h * 4) as usize];
	match i.fmt {
		Fmt::Bc(n) => block_compression::decode::decompress_blocks_as_rgba8(variant(n), i.w, i.h, data, &mut rgba),
		Fmt::Rgba => rgba.copy_from_slice(data),
		Fmt::Bgra => {
			for (o, p) in rgba.chunks_exact_mut(4).zip(data.chunks_exact(4)) {
				o.copy_from_slice(&[p[2], p[1], p[0], p[3]]);
			}
		}
	}
	let img = RgbaImage::from_raw(i.w, i.h, rgba).ok_or(tr("ошибка размера", "size error"))?;
	let mut png = Vec::new();
	img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageOutputFormat::Png).map_err(|e| lformat!("ошибка PNG: {e}", "PNG error: {e}"))?;
	Ok(Decoded { png })
}

fn encode_level(img: &RgbaImage, fmt: Fmt) -> Vec<u8> {
	match fmt {
		Fmt::Rgba => img.as_raw().clone(),
		Fmt::Bgra => img.as_raw().chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect(),
		Fmt::Bc(n) => {
			let (w, h) = (img.width(), img.height());
			let (pw, ph) = (w.div_ceil(4) * 4, h.div_ceil(4) * 4);
			let padded = if (pw, ph) == (w, h) {
				img.clone()
			} else {
				RgbaImage::from_fn(pw, ph, |x, y| *img.get_pixel(x.min(w - 1), y.min(h - 1)))
			};
			let v = variant(n);
			let mut out = vec![0u8; v.blocks_byte_size(pw, ph)];
			block_compression::encode::compress_rgba8(v, padded.as_raw(), &mut out, pw, ph, pw * 4);
			out
		}
	}
}

pub fn compile(png: &[u8], orig: &[u8]) -> Result<(Vec<u8>, Option<String>), String> {
	let i = info(orig)?;
	let img = image::load_from_memory_with_format(png, image::ImageFormat::Png).map_err(|e| lformat!("не удалось прочитать PNG: {e}", "could not read the PNG: {e}"))?.to_rgba8();
	let (w, h) = (img.width(), img.height());
	let mut warn = None;
	if (w, h) != (i.w, i.h) {
		warn = Some(lformat!("размер изменён: было {}x{}, стало {w}x{h}", "size changed: was {}x{}, now {w}x{h}", i.w, i.h));
	}
	let mut out = orig[..i.data_off].to_vec();
	out[12..16].copy_from_slice(&h.to_le_bytes());
	out[16..20].copy_from_slice(&w.to_le_bytes());
	if w > 16384 || h > 16384 {
		return Err(lformat!("PNG слишком большой: {w}x{h}", "the PNG is too large: {w}x{h}"));
	}
	let pitch_flag = u32at(orig, 8) & 0x8 != 0;
	let f20 = if pitch_flag && !matches!(i.fmt, Fmt::Bc(_)) { w * 4 } else { level_size(i.fmt, w, h) as u32 };
	out[20..24].copy_from_slice(&f20.to_le_bytes());
	let max_mips = 32 - w.max(h).leading_zeros();
	let mips = i.mips.min(max_mips).max(1);
	if u32at(orig, 28) != 0 {
		out[28..32].copy_from_slice(&mips.to_le_bytes());
	}
	for m in 0..mips {
		let (lw, lh) = ((w >> m).max(1), (h >> m).max(1));
		let lvl = if m == 0 { img.clone() } else { image::imageops::resize(&img, lw, lh, image::imageops::FilterType::Triangle) };
		out.extend(encode_level(&lvl, i.fmt));
	}
	Ok((out, warn))
}
