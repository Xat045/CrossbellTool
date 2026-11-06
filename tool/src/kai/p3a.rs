#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use std::path::Path;
use xxhash_rust::xxh64::xxh64;

#[derive(Clone, Debug)]
pub struct Entry {
	pub name: String,
	pub cmp_type: u64,
	pub cmp_size: u64,
	pub unc_size: u64,
	pub offset: u64,
	pub unc_hash: u64,
}

pub struct Archive {
	pub path: std::path::PathBuf,
	pub entries: Vec<Entry>,
	dict: Option<Vec<u8>>,
	pub label: String,
	head: Vec<u8>,
	toc: Vec<u8>,
	esize: usize,
	dict_block: Option<Vec<u8>>,
}

fn u32at(d: &[u8], o: usize) -> Result<u32, String> {
	d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| tr("архив обрезан", "the archive is truncated").to_string())
}
fn u64at(d: &[u8], o: usize) -> Result<u64, String> {
	d.get(o..o + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| tr("архив обрезан", "the archive is truncated").to_string())
}

impl Archive {
	pub fn open(path: &Path) -> Result<Archive, String> {
		use std::io::{Read, Seek, SeekFrom};
		let err = |e: std::io::Error| format!("{}: {e}", path.display());
		let mut f = std::fs::File::open(path).map_err(err)?;
		let mut head = [0u8; 0x30];
		f.read_exact(&mut head).map_err(err)?;
		if !head.starts_with(b"PH3ARCV\0") {
			return Err(lformat!("{}: это не архив P3A", "{}: this is not a P3A archive", path.display()));
		}
		let flags = u32at(&head, 8)?;
		let version = u32at(&head, 12)?;
		let n = u64at(&head, 16)? as usize;
		let (toc_start, esize) = if version >= 1200 { (0x30u64, u32at(&head, 0x2c)? as usize) } else { (0x20u64, 0x128usize) };
		let file_len = f.metadata().map(|m| m.len()).unwrap_or(u64::MAX);
		if n > 10_000_000 || esize < 0x120 || esize > 0x1000 || toc_start + (n * esize) as u64 > file_len {
			return Err(lformat!("{}: повреждённое оглавление", "{}: damaged table of contents", path.display()));
		}
		f.seek(SeekFrom::Start(toc_start)).map_err(err)?;
		let mut toc = vec![0u8; n * esize];
		f.read_exact(&mut toc).map_err(err)?;
		let mut entries = Vec::with_capacity(n);
		for k in 0..n {
			let raw = &toc[k * esize..(k + 1) * esize];
			let name_end = raw[..0x100].iter().position(|&b| b == 0).unwrap_or(0x100);
			entries.push(Entry {
				name: String::from_utf8_lossy(&raw[..name_end]).replace('\\', "/"),
				cmp_type: u64at(raw, 0x100)?,
				cmp_size: u64at(raw, 0x108)?,
				unc_size: u64at(raw, 0x110)?,
				offset: u64at(raw, 0x118)?,
				unc_hash: u64at(raw, 0x128).unwrap_or(0),
			});
		}
		let mut dict_block = None;
		let dict = if flags & 1 != 0 {
			let mut m = [0u8; 16];
			f.read_exact(&mut m).map_err(err)?;
			if &m[..8] == b"P3ADICT\0" {
				let size = u64at(&m, 8)? as usize;
				let mut d = vec![0u8; size];
				f.read_exact(&mut d).map_err(err)?;
				let mut blk = m.to_vec();
				blk.extend_from_slice(&d);
				dict_block = Some(blk);
				Some(d)
			} else {
				None
			}
		} else {
			None
		};
		let head_len = if version >= 1200 { 0x30 } else { 0x20 };
		Ok(Archive { path: path.to_path_buf(), entries, dict, label: String::new(), head: head[..head_len].to_vec(), toc, esize, dict_block })
	}

	pub fn read(&self, e: &Entry) -> Result<Vec<u8>, String> {
		use std::io::{Read, Seek, SeekFrom};
		let err = |x: std::io::Error| format!("{}: {x}", self.path.display());
		if e.unc_size == 0 {
			return Ok(Vec::new());
		}
		let mut f = std::fs::File::open(&self.path).map_err(err)?;
		let file_len = f.metadata().map(|m| m.len()).unwrap_or(u64::MAX);
		if e.offset.saturating_add(e.cmp_size) > file_len {
			return Err(lformat!("{}: запись выходит за конец архива (архив повреждён или обрезан)", "{}: an entry goes past the end of the archive (damaged or truncated)", e.name));
		}
		f.seek(SeekFrom::Start(e.offset)).map_err(err)?;
		let mut raw = vec![0u8; e.cmp_size as usize];
		f.read_exact(&mut raw).map_err(err)?;
		match e.cmp_type {
			0 => Ok(raw),
			1 => lz4_flex::block::decompress(&raw, e.unc_size as usize).map_err(|x| lformat!("{}: ошибка LZ4: {x}", "{}: LZ4 error: {x}", e.name)),
			2 | 3 => {
				let mut dec = ruzstd::decoding::FrameDecoder::new();
				if e.cmp_type == 3 {
					let d = self.dict.as_ref().ok_or(tr("нет словаря zstd", "no zstd dictionary"))?;
					let dict = ruzstd::decoding::Dictionary::decode_dict(d).map_err(|x| lformat!("словарь zstd: {x}", "zstd dictionary: {x}"))?;
					dec.add_dict(dict).map_err(|x| lformat!("словарь zstd: {x}", "zstd dictionary: {x}"))?;
				}
				let mut out = Vec::with_capacity(e.unc_size as usize);
				let mut r = ruzstd::decoding::StreamingDecoder::new_with_decoder(&raw[..], dec).map_err(|x| lformat!("{}: ошибка zstd: {x}", "{}: zstd error: {x}", e.name))?;
				r.read_to_end(&mut out).map_err(|x| lformat!("{}: ошибка zstd: {x}", "{}: zstd error: {x}", e.name))?;
				Ok(out)
			}
			t => Err(lformat!("{}: неизвестное сжатие {t}", "{}: unknown compression {t}", e.name)),
		}
	}
}

pub fn verify<'a>(path: &Path, expect: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Result<usize, String> {
	let a = Archive::open(path)?;
	let by: std::collections::HashMap<String, &Entry> = a.entries.iter().map(|e| (e.name.to_lowercase().replace('\\', "/"), e)).collect();
	let mut n = 0;
	for (name, data) in expect {
		let e = by.get(&name.to_lowercase().replace('\\', "/")).ok_or_else(|| lformat!("{name}: нет в архиве", "{name}: not in the archive"))?;
		if a.read(e)? != data {
			return Err(lformat!("{name}: в архиве не то, что собиралось", "{name}: the archive holds something other than what was built"));
		}
		n += 1;
	}
	Ok(n)
}

#[derive(Clone, Copy, Debug)]
pub struct Spooled {
	offset: u64,
	cmp_size: u64,
	unc_size: u64,
	cmp_hash: u64,
	unc_hash: u64,
}

pub struct Spool {
	path: std::path::PathBuf,
	file: std::sync::Mutex<(std::fs::File, u64)>,
}

impl Spool {
	pub fn create(path: &Path) -> Result<Spool, String> {
		let f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
		Ok(Spool { path: path.to_path_buf(), file: std::sync::Mutex::new((f, 0)) })
	}

	pub fn put(&self, data: &[u8]) -> Result<Spooled, String> {
		use std::io::Write;
		let c = lz4_flex::block::compress(data);
		let cmp_hash = xxh64(&c, 0);
		let unc_hash = xxh64(data, 0);
		let mut g = self.file.lock().unwrap();
		let offset = g.1;
		g.0.write_all(&c).map_err(|e| format!("{}: {e}", self.path.display()))?;
		g.1 += c.len() as u64;
		Ok(Spooled { offset, cmp_size: c.len() as u64, unc_size: data.len() as u64, cmp_hash, unc_hash })
	}
}

impl Drop for Spool {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.path);
	}
}

pub fn verify_spooled<'a>(path: &Path, expect: impl IntoIterator<Item = (&'a str, Spooled)>) -> Result<usize, String> {
	let a = Archive::open(path)?;
	let by: std::collections::HashMap<String, &Entry> = a.entries.iter().map(|e| (e.name.to_lowercase().replace('\\', "/"), e)).collect();
	let mut n = 0;
	for (name, s) in expect {
		let e = by.get(&name.to_lowercase().replace('\\', "/")).ok_or_else(|| lformat!("{name}: нет в архиве", "{name}: not in the archive"))?;
		let d = a.read(e)?;
		if d.len() as u64 != s.unc_size || xxh64(&d, 0) != s.unc_hash {
			return Err(lformat!("{name}: в архиве не то, что собиралось", "{name}: the archive holds something other than what was built"));
		}
		n += 1;
	}
	Ok(n)
}

pub fn write(files: &[(String, Vec<u8>)]) -> Vec<u8> {
	let n = files.len();
	let header_len = (48 + 304 * n).div_ceil(64) * 64;
	let mut blocks: Vec<Vec<u8>> = Vec::with_capacity(n);
	let mut offsets = Vec::with_capacity(n);
	let mut pos = header_len;
	for (_, d) in files {
		let c = lz4_flex::block::compress(d);
		offsets.push(pos);
		pos += c.len();
		pos = pos.div_ceil(64) * 64;
		blocks.push(c);
	}
	let mut h = Vec::with_capacity(header_len);
	h.extend_from_slice(b"PH3ARCV\0");
	h.extend_from_slice(&0u32.to_le_bytes());
	h.extend_from_slice(&1200u32.to_le_bytes());
	h.extend_from_slice(&(n as u64).to_le_bytes());
	let hh = xxh64(&h, 0);
	h.extend_from_slice(&hh.to_le_bytes());
	let mut ext = Vec::new();
	ext.extend_from_slice(&16u32.to_le_bytes());
	ext.extend_from_slice(&304u32.to_le_bytes());
	h.extend_from_slice(&xxh64(&ext, 0).to_le_bytes());
	h.extend_from_slice(&ext);
	for (i, (name, d)) in files.iter().enumerate() {
		let mut nm = name.to_lowercase().replace('\\', "/").into_bytes();
		nm.resize(256, 0);
		h.extend_from_slice(&nm);
		h.extend_from_slice(&1u64.to_le_bytes());
		h.extend_from_slice(&(blocks[i].len() as u64).to_le_bytes());
		h.extend_from_slice(&(d.len() as u64).to_le_bytes());
		h.extend_from_slice(&(offsets[i] as u64).to_le_bytes());
		h.extend_from_slice(&xxh64(&blocks[i], 0).to_le_bytes());
		h.extend_from_slice(&xxh64(d, 0).to_le_bytes());
	}
	h.resize(header_len, 0);
	let mut out = h;
	for (i, b) in blocks.iter().enumerate() {
		out.resize(offsets[i], 0);
		out.extend_from_slice(b);
	}
	out
}

pub fn rebuild(src: &Archive, replace: &std::collections::HashMap<String, Spooled>, spool: &Spool, out: &Path) -> Result<(usize, usize), String> {
	let esize = src.esize;
	let mut toc = src.toc.clone();
	let mut names_seen = std::collections::HashSet::new();
	let mut new_blocks: Vec<Option<Spooled>> = Vec::with_capacity(src.entries.len());
	let mut replaced = 0;
	let set = |raw: &mut [u8], s: &Spooled| {
		raw[0x100..0x108].copy_from_slice(&1u64.to_le_bytes());
		raw[0x108..0x110].copy_from_slice(&s.cmp_size.to_le_bytes());
		raw[0x110..0x118].copy_from_slice(&s.unc_size.to_le_bytes());
		if esize >= 0x128 {
			raw[0x120..0x128].copy_from_slice(&s.cmp_hash.to_le_bytes());
		}
		if esize >= 0x130 {
			raw[0x128..0x130].copy_from_slice(&s.unc_hash.to_le_bytes());
		}
	};
	for (i, e) in src.entries.iter().enumerate() {
		let key = e.name.to_lowercase();
		names_seen.insert(key.clone());
		if let Some(s) = replace.get(&key) {
			set(&mut toc[i * esize..(i + 1) * esize], s);
			new_blocks.push(Some(*s));
			replaced += 1;
		} else {
			new_blocks.push(None);
		}
	}
	let mut added: Vec<(&String, &Spooled)> = replace.iter().filter(|(k, _)| !names_seen.contains(*k)).collect();
	added.sort_by(|a, b| a.0.cmp(b.0));
	for (name, s) in &added {
		let mut raw = vec![0u8; esize];
		let mut nm = name.as_bytes().to_vec();
		nm.truncate(0xff);
		raw[..nm.len()].copy_from_slice(&nm);
		set(&mut raw, s);
		toc.extend_from_slice(&raw);
		new_blocks.push(Some(**s));
		replaced += 1;
	}
	let n = new_blocks.len();
	let mut head = src.head.clone();
	if n != src.entries.len() {
		head[16..24].copy_from_slice(&(n as u64).to_le_bytes());
		let hh = xxh64(&head[..0x18], 0);
		head[0x18..0x20].copy_from_slice(&hh.to_le_bytes());
	}
	let dict_len = src.dict_block.as_ref().map(|d| d.len()).unwrap_or(0);
	let header_len = (head.len() + toc.len() + dict_len).div_ceil(64) * 64;
	let mut pos = header_len as u64;
	for i in 0..n {
		let size = u64::from_le_bytes(toc[i * esize + 0x108..i * esize + 0x110].try_into().unwrap());
		toc[i * esize + 0x118..i * esize + 0x120].copy_from_slice(&pos.to_le_bytes());
		pos += size;
		pos = pos.div_ceil(64) * 64;
	}
	let tmp = out.with_extension("p3a.part");
	let res = write_rebuilt(src, &head, &toc, &new_blocks, spool, &tmp);
	if let Err(e) = res {
		let _ = std::fs::remove_file(&tmp);
		return Err(e);
	}
	std::fs::rename(&tmp, out).map_err(|e| {
		let _ = std::fs::remove_file(&tmp);
		format!("{}: {e}", out.display())
	})?;
	Ok((n, replaced))
}

fn write_rebuilt(src: &Archive, head: &[u8], toc: &[u8], new_blocks: &[Option<Spooled>], spool: &Spool, tmp: &Path) -> Result<(), String> {
	use std::io::{Read, Seek, SeekFrom, Write};
	let esize = src.esize;
	let n = new_blocks.len();
	let err = |e: std::io::Error| format!("{}: {e}", tmp.display());
	let mut w = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(tmp).map_err(err)?);
	let mut written = 0u64;
	let put = |w: &mut std::io::BufWriter<std::fs::File>, b: &[u8], written: &mut u64| -> Result<(), String> {
		w.write_all(b).map_err(|e| format!("{}: {e}", tmp.display()))?;
		*written += b.len() as u64;
		Ok(())
	};
	put(&mut w, &head, &mut written)?;
	put(&mut w, &toc, &mut written)?;
	if let Some(d) = &src.dict_block {
		put(&mut w, d, &mut written)?;
	}
	let mut f = std::fs::File::open(&src.path).map_err(|e| format!("{}: {e}", src.path.display()))?;
	let mut sp = std::fs::File::open(&spool.path).map_err(|e| format!("{}: {e}", spool.path.display()))?;
	let mut buf = vec![0u8; 1 << 20];
	for i in 0..n {
		let off = u64::from_le_bytes(toc[i * esize + 0x118..i * esize + 0x120].try_into().unwrap());
		if off > written {
			put(&mut w, &vec![0u8; (off - written) as usize], &mut written)?;
		}
		let (from, from_path, start, size) = match &new_blocks[i] {
			Some(s) => (&mut sp, &spool.path, s.offset, s.cmp_size),
			None => (&mut f, &src.path, src.entries[i].offset, src.entries[i].cmp_size),
		};
		from.seek(SeekFrom::Start(start)).map_err(|x| format!("{}: {x}", from_path.display()))?;
		let mut left = size as usize;
		while left > 0 {
			let k = left.min(buf.len());
			from.read_exact(&mut buf[..k]).map_err(|x| format!("{}: {x}", from_path.display()))?;
			put(&mut w, &buf[..k], &mut written)?;
			left -= k;
		}
	}
	w.flush().map_err(err)?;
	drop(w);
	drop(f);
	drop(sp);
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn write_open_read_rebuild() {
		let d = std::env::temp_dir().join(format!("p3a_test_{}", std::process::id()));
		std::fs::create_dir_all(&d).unwrap();
		let files = vec![("a/x.bin".to_string(), (0..200u8).collect::<Vec<u8>>()), ("b.txt".to_string(), b"hello".to_vec()), ("empty".to_string(), Vec::new())];
		let p = d.join("t.p3a");
		std::fs::write(&p, write(&files)).unwrap();
		let a = Archive::open(&p).unwrap();
		assert_eq!(a.entries.len(), 3);
		for (e, (n, data)) in a.entries.iter().zip(&files) {
			assert_eq!(&e.name, n);
			assert_eq!(&a.read(e).unwrap(), data);
			assert_eq!(e.unc_hash, xxh64(data, 0));
		}
		let spool_path = d.join("spool.tmp");
		let spool = Spool::create(&spool_path).unwrap();
		let other = spool.put(b"other!").unwrap();
		let mut rep = std::collections::HashMap::new();
		rep.insert("b.txt".to_string(), spool.put(b"world!").unwrap());
		rep.insert("new.bin".to_string(), spool.put(b"added").unwrap());
		let out = d.join("r.p3a");
		assert_eq!(rebuild(&a, &rep, &spool, &out).unwrap(), (4, 2));
		let b = Archive::open(&out).unwrap();
		assert_eq!(b.read(&b.entries[1]).unwrap(), b"world!");
		assert_eq!(b.read(&b.entries[0]).unwrap(), files[0].1);
		assert_eq!(b.read(&b.entries[3]).unwrap(), b"added");
		assert_eq!(verify(&p, files.iter().map(|(n, v)| (n.as_str(), v.as_slice()))).unwrap(), 3);
		assert_eq!(verify_spooled(&out, rep.iter().map(|(n, v)| (n.as_str(), *v))).unwrap(), 2);
		assert!(verify_spooled(&out, [("b.txt", other)]).is_err());
		drop(spool);
		assert!(!spool_path.exists());
		assert!(verify(&out, [("b.txt", &b"other!"[..])]).is_err());
		assert!(verify(&out, [("nope.bin", &b""[..])]).is_err());
		let mut raw = std::fs::read(&p).unwrap();
		let off = a.entries[0].offset as usize;
		raw[off] ^= 0xff;
		std::fs::write(&p, raw).unwrap();
		assert!(verify(&p, files.iter().map(|(n, v)| (n.as_str(), v.as_slice()))).is_err());
		let _ = std::fs::remove_dir_all(&d);
	}
}
