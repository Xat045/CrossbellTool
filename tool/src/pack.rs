#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

pub const FILE: &str = "data.dat";
const MAGIC: &[u8; 8] = b"KZPAK1\0\0";
const KEY: [u8; 32] = *b"\x9e\x37\x79\xb9\x7f\x4a\x7c\x15\xf3\x9c\xc0\x60\x5c\xed\xc8\x34\x10\x82\x27\x6b\xf3\xa2\x72\x37\x4c\x8f\x1a\xd5\x63\x2b\x09\xe1";

fn xor(data: &mut [u8], nonce: u64) {
	let mut buf = [0u8; 40];
	buf[..32].copy_from_slice(&KEY);
	for (k, chunk) in data.chunks_mut(8).enumerate() {
		buf[32..].copy_from_slice(&(k as u64).to_le_bytes());
		let ks = xxhash_rust::xxh64::xxh64(&buf, nonce).to_le_bytes();
		for (b, s) in chunk.iter_mut().zip(ks) {
			*b ^= s;
		}
	}
}

pub fn build(dir: &Path, out: &Path) -> Result<usize, String> {
	let mut files = Vec::new();
	fn walk(d: &Path, pre: &str, out: &mut Vec<(String, std::path::PathBuf)>) -> std::io::Result<()> {
		let mut v: Vec<_> = std::fs::read_dir(d)?.collect::<Result<_, _>>()?;
		v.sort_by_key(|e| e.file_name());
		for e in v {
			let name = format!("{pre}{}", e.file_name().to_string_lossy());
			if e.file_type()?.is_dir() {
				walk(&e.path(), &format!("{name}/"), out)?;
			} else {
				out.push((name, e.path()));
			}
		}
		Ok(())
	}
	walk(dir, "", &mut files).map_err(|e| format!("{}: {e}", dir.display()))?;
	let mut body = Vec::new();
	body.extend((files.len() as u32).to_le_bytes());
	for (name, p) in &files {
		let data = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
		body.extend((name.len() as u16).to_le_bytes());
		body.extend(name.as_bytes());
		body.extend((data.len() as u32).to_le_bytes());
		body.extend(data);
	}
	let sum = xxhash_rust::xxh64::xxh64(&body, 0);
	body.extend(sum.to_le_bytes());
	let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
	xor(&mut body, nonce);
	let mut out_b = MAGIC.to_vec();
	out_b.extend(nonce.to_le_bytes());
	out_b.extend(body);
	std::fs::write(out, out_b).map_err(|e| format!("{}: {e}", out.display()))?;
	Ok(files.len())
}

pub fn parse(raw: &[u8]) -> Option<HashMap<String, Vec<u8>>> {
	if raw.len() < 28 || &raw[..8] != MAGIC {
		return None;
	}
	let nonce = u64::from_le_bytes(raw[8..16].try_into().ok()?);
	let mut body = raw[16..].to_vec();
	xor(&mut body, nonce);
	let (body, sum) = body.split_at(body.len() - 8);
	if xxhash_rust::xxh64::xxh64(body, 0).to_le_bytes() != sum {
		return None;
	}
	fn take<'a>(b: &'a [u8], o: &mut usize, n: usize) -> Option<&'a [u8]> {
		let s = b.get(*o..*o + n)?;
		*o += n;
		Some(s)
	}
	let mut o = 0usize;
	let count = u32::from_le_bytes(take(body, &mut o, 4)?.try_into().ok()?);
	let mut out = HashMap::new();
	for _ in 0..count {
		let nl = u16::from_le_bytes(take(body, &mut o, 2)?.try_into().ok()?) as usize;
		let name = String::from_utf8(take(body, &mut o, nl)?.to_vec()).ok()?;
		let dl = u32::from_le_bytes(take(body, &mut o, 4)?.try_into().ok()?) as usize;
		out.insert(name, take(body, &mut o, dl)?.to_vec());
	}
	Some(out)
}

fn all() -> &'static HashMap<String, Vec<u8>> {
	static P: OnceLock<HashMap<String, Vec<u8>>> = OnceLock::new();
	P.get_or_init(|| std::fs::read(std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join(FILE))).unwrap_or_default()).ok().and_then(|b| parse(&b)).unwrap_or_default())
}

pub fn get(name: &str) -> Option<&'static [u8]> {
	all().get(name).map(|v| v.as_slice())
}

pub fn has_game(g: &str) -> bool {
	let pre = format!("{g}/");
	all().keys().any(|k| k.starts_with(&pre))
}

#[cfg(test)]
#[test]
fn roundtrip() {
	let d = std::env::temp_dir().join(format!("kzpak_test_{}", std::process::id()));
	std::fs::create_dir_all(d.join("Zero")).unwrap();
	std::fs::write(d.join("Zero").join("a.png"), b"PNG data").unwrap();
	std::fs::write(d.join("b.bin"), vec![7u8; 1000]).unwrap();
	let out = d.with_extension("dat");
	assert_eq!(build(&d, &out).unwrap(), 2);
	let raw = std::fs::read(&out).unwrap();
	assert!(!raw.windows(8).any(|w| w == b"PNG data") && !raw.windows(5).any(|w| w == b"a.png"));
	let m = parse(&raw).unwrap();
	assert_eq!(m["Zero/a.png"], b"PNG data");
	assert_eq!(m["b.bin"], vec![7u8; 1000]);
	let mut bad = raw.clone();
	bad[40] ^= 1;
	assert!(parse(&bad).is_none());
	let _ = std::fs::remove_dir_all(&d);
	let _ = std::fs::remove_file(&out);
}
