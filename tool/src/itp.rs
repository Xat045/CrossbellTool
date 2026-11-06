#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use cradle::itp32::{self, Itp32};
use image::RgbaImage;

pub struct Decoded {
	pub png: Vec<u8>,
	pub width: usize,
	pub height: usize,
	pub mips: usize,
	pub old: bool,
}

fn to_png(img: &RgbaImage) -> Result<Vec<u8>, String> {
	let mut png = Vec::new();
	img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageOutputFormat::Png)
		.map_err(|e| lformat!("ошибка записи PNG: {e}", "PNG write error: {e}"))?;
	Ok(png)
}

fn decompile_old(data: &[u8]) -> Result<Decoded, String> {
	let itp = std::panic::catch_unwind(|| cradle::itp::read(data))
		.map_err(|_| tr("старый формат ITP (с палитрой) не читается: файл повреждён", "can't read the old ITP format (with a palette): the file is corrupted").to_string())?
		.map_err(|e| lformat!("старый формат ITP (с палитрой) не читается: {e}", "can't read the old ITP format (with a palette): {e}"))?;
	let colors = itp.palette.len();
	if colors == 0 {
		return Err(tr("старый формат ITP (с палитрой): палитра лежит в другом файле", "old ITP format (with a palette): the palette is in another file").into());
	}
	if itp.image.pixels().any(|p| p.0[0] as usize >= colors) {
		return Err(tr("старый формат ITP (с палитрой): номер цвета вне палитры", "old ITP format (with a palette): color index outside the palette").into());
	}
	let img = itp.to_rgba();
	Ok(Decoded { png: to_png(&img)?, width: img.width() as usize, height: img.height() as usize, mips: 0, old: true })
}

pub fn decompile(data: &[u8]) -> Result<Decoded, String> {
	if !data.starts_with(b"ITP\xFF") {
		return decompile_old(data);
	}
	let itp = itp32::read(data).map_err(|e| lformat!("не удалось прочитать ITP: {e}", "could not read the ITP: {e}"))?;
	let img = itp.to_rgba(0);
	Ok(Decoded { png: to_png(&img)?, width: itp.width, height: itp.height, mips: itp.levels().saturating_sub(1), old: false })
}

pub fn to_dds(data: &[u8]) -> Result<Decoded, String> {
	if !data.starts_with(b"ITP\xFF") {
		return Err(tr("старый формат ITP (с палитрой) не поддерживается", "the old ITP format (with a palette) is not supported").into());
	}
	let itp = itp32::read(data).map_err(|e| lformat!("не удалось прочитать ITP: {e}", "could not read the ITP: {e}"))?;
	let n = itp.levels().max(1) as u32;
	let level0 = itp.levels.first().map_or(0, |l| l.len() * 16) as u32;
	let mut d = b"DDS ".to_vec();
	let mut u = |v: u32| d.extend(v.to_le_bytes());
	u(124);
	u(0x1 | 0x2 | 0x4 | 0x1000 | 0x80000 | if n > 1 { 0x20000 } else { 0 });
	u(itp.height as u32);
	u(itp.width as u32);
	u(level0);
	u(0);
	u(n);
	for _ in 0..11 {
		u(0);
	}
	u(32);
	u(0x4);
	u(u32::from_le_bytes(*b"DX10"));
	for _ in 0..5 {
		u(0);
	}
	u(0x1000 | if n > 1 { 0x8 | 0x40_0000 } else { 0 });
	for _ in 0..4 {
		u(0);
	}
	for v in [98, 3, 0, 1, 0] {
		u(v);
	}
	d.extend(itp.levels.iter().flatten().flat_map(|b| b.to_le_bytes()));
	Ok(Decoded { png: d, width: itp.width, height: itp.height, mips: itp.levels().saturating_sub(1), old: false })
}

pub fn compile_dds(dds: &[u8], mips: usize, expect: Option<(usize, usize)>) -> Result<(Vec<u8>, Option<String>), String> {
	let u = |o: usize| dds.get(o..o + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()));
	if !dds.starts_with(b"DDS ") || dds.len() < 148 {
		return Err(tr("это не DDS", "this is not a DDS").into());
	}
	let (h, w, n) = (u(12) as usize, u(16) as usize, (u(28) as usize).max(1));
	let warn = expect.filter(|&e| e != (w, h)).map(|(ew, eh)| lformat!("размер изменён: было {ew}x{eh}, стало {w}x{h}", "size changed: was {ew}x{eh}, now {w}x{h}"));
	if &dds[84..88] == b"DX10" && (97..=99).contains(&u(128)) && w % 4 == 0 && h % 4 == 0 {
		let mut it = dds[148..].chunks_exact(16).map(|a| u128::from_le_bytes(a.try_into().unwrap()));
		let mut levels = Vec::new();
		for l in 0..n {
			let blocks = ((w >> l) * (h >> l)) >> 4;
			if blocks == 0 {
				break;
			}
			let lv: Vec<u128> = it.by_ref().take(blocks).collect();
			if lv.len() < blocks {
				return Err(tr("DDS обрезан", "the DDS is truncated").into());
			}
			levels.push(lv);
		}
		let data = itp32::write(&Itp32 { width: w, height: h, levels }).map_err(|e| lformat!("ошибка записи ITP: {e}", "ITP write error: {e}"))?;
		return Ok((data, warn));
	}
	let png = crate::kai::dds::decompile(dds)?.png;
	compile(&png, mips, expect)
}

fn encode_level(img: &RgbaImage) -> Vec<u128> {
	use block_compression::{CompressionVariant, BC7Settings, encode::compress_rgba8};
	let (w, h) = (img.width(), img.height());
	let variant = CompressionVariant::BC7(BC7Settings::alpha_fast());
	let mut blocks = vec![0u8; variant.blocks_byte_size(w, h)];
	compress_rgba8(variant, img.as_raw(), &mut blocks, w, h, w * 4);
	blocks.chunks_exact(16).map(|a| u128::from_le_bytes(a.try_into().unwrap())).collect()
}

pub fn compile(png: &[u8], mips: usize, expect: Option<(usize, usize)>) -> Result<(Vec<u8>, Option<String>), String> {
	let img = image::load_from_memory_with_format(png, image::ImageFormat::Png)
		.map_err(|e| lformat!("не удалось прочитать PNG: {e}", "could not read the PNG: {e}"))?
		.to_rgba8();
	let (w, h) = (img.width() as usize, img.height() as usize);
	if w % 4 != 0 || h % 4 != 0 {
		return Err(lformat!("размер {w}x{h}: ширина и высота должны делиться на 4", "size {w}x{h}: width and height must be divisible by 4"));
	}
	let mut warn = None;
	if let Some((ew, eh)) = expect {
		if (ew, eh) != (w, h) {
			warn = Some(lformat!("размер изменён: было {ew}x{eh}, стало {w}x{h}", "size changed: was {ew}x{eh}, now {w}x{h}"));
		}
	}
	let mut levels = vec![encode_level(&img)];
	for n in 1..=mips {
		let (lw, lh) = (w >> n, h >> n);
		if lw < 4 || lh < 4 || lw % 4 != 0 || lh % 4 != 0 { break }
		let small = image::imageops::resize(&img, lw as u32, lh as u32, image::imageops::FilterType::Triangle);
		levels.push(encode_level(&small));
	}
	let itp = Itp32 { width: w, height: h, levels };
	let data = itp32::write(&itp).map_err(|e| lformat!("ошибка записи ITP: {e}", "ITP write error: {e}"))?;
	Ok((data, warn))
}

fn itc_frame(data: &[u8], shared: Option<&[image::Rgba<u8>]>) -> Result<RgbaImage, String> {
	if data.starts_with(b"ITP\xFF") {
		let itp = itp32::read(data).map_err(|e| lformat!("не удалось прочитать ITP: {e}", "could not read the ITP: {e}"))?;
		return Ok(itp.to_rgba(0));
	}
	let mut itp = std::panic::catch_unwind(|| cradle::itp::read(data))
		.map_err(|_| tr("кадр повреждён", "the frame is corrupted").to_string())?
		.map_err(|e| lformat!("кадр не читается: {e}", "can't read the frame: {e}"))?;
	if let Some(p) = shared {
		itp.palette = p.to_vec();
	}
	let colors = itp.palette.len();
	if colors == 0 || itp.image.pixels().any(|p| p.0[0] as usize >= colors) {
		return Err(tr("номер цвета вне палитры", "color index outside the palette").into());
	}
	Ok(itp.to_rgba())
}

pub fn itc_decode(data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
	let itc = cradle::itc::read(data).map_err(|e| lformat!("не удалось прочитать ITC: {e}", "can't read the ITC: {e}"))?;
	itc.content.iter().map(|c| to_png(&itc_frame(c, itc.palette.as_deref())?)).collect()
}

pub fn itc_replace(orig: &[u8], edited: &[(usize, Vec<u8>)]) -> Result<Vec<u8>, String> {
	let mut itc = cradle::itc::read(orig).map_err(|e| lformat!("не удалось прочитать ITC игры: {e}", "can't read the game's ITC: {e}"))?;
	for (i, png) in edited {
		let Some(old) = itc.content.get(*i) else { return Err(lformat!("в файле игры нет кадра {i}", "the game file has no frame {i}")) };
		let was = itc_frame(old, itc.palette.as_deref())?;
		let (data, _) = compile(png, 0, Some((was.width() as usize, was.height() as usize)))?;
		let now = itp32::read(&data).map_err(|e| lformat!("не удалось прочитать ITP: {e}", "could not read the ITP: {e}"))?;
		if (now.width as u32, now.height as u32) != (was.width(), was.height()) {
			return Err(lformat!("кадр {i}: размер изменён, было {}x{}: у спрайта смещения считаются от размера кадра", "frame {i}: the size changed, it was {}x{}: sprite offsets depend on the frame size", was.width(), was.height()));
		}
		itc.content[*i] = data.into();
	}
	cradle::itc::write(&itc).map_err(|e| lformat!("ошибка записи ITC: {e}", "error writing the ITC: {e}"))
}
