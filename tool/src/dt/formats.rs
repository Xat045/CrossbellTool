#[allow(unused_imports)]
use crate::{leprintln, lformat, lprintln};
#[allow(unused_imports)]
use crate::lang::tr;
use super::model::{Disc, Res};

pub type Rule = fn(&mut Disc) -> Res<()>;

pub struct Format {
	pub name: &'static str,
	pub rule: Rule,
}

pub fn find(stem: &str) -> Option<Format> {
	let s = stem.to_ascii_lowercase();
	let f: (&'static str, Rule) = match s.as_str() {
		"t_town" => ("town", town),
		"t_name" => ("name", name),
		"t_dbmon" => ("dbmon", name),
		"t_memo" => ("memo", str_table),
		"t_ittxt" | "t_ittxt2" => ("ittxt", ittxt),
		"t_quest" => ("quest", quest),
		"t_fish" => ("fish", fish_any),
		"t_record" => ("record", record),
		"t_magic" => ("magic", magic),
		"t_exmov" => ("exmov", exmov),
		"t_exvis" | "t_exchr" => ("extra", extra_any),
		"t_cook" => ("cook", cook_any),
		"t_shop" => ("shop", shop),
		"t_mgame" => ("mgame", mgame),
		"t_mstqrt" => ("mstqrt", mstqrt),
		"monsnote" | "monsnote_us" => ("monsnote", monsnote),
		_ if s.len() == 8 && s.starts_with("t_book") && s[6..].bytes().all(|c| c.is_ascii_digit()) => ("book", str_table),
		_ if s.len() > 7 && s.starts_with("b_story") && s[7..].bytes().all(|c| c.is_ascii_digit()) => ("story", story),
		_ => return None,
	};
	Some(Format { name: f.0, rule: f.1 })
}

fn span(a: usize, b: usize) -> Res<usize> {
	b.checked_sub(a).ok_or_else(|| lformat!("неожиданный порядок разделов (0x{a:X} > 0x{b:X})", "unexpected section order (0x{a:X} > 0x{b:X})"))
}

fn str_table(d: &mut Disc) -> Res<()> {
	for t in d.ptr_table(0, usize::MAX)? {
		d.string(t)?;
	}
	Ok(())
}

fn story(d: &mut Disc) -> Res<()> {
	let mut ts = d.ptr_table(0, usize::MAX)?;
	ts.sort();
	ts.dedup();
	let mut rest = &ts[..];
	if let [a, b, ..] = ts[..] {
		if !d.bytes(a, b)?.contains(&0) {
			d.data(a, b - a)?;
			rest = &ts[1..];
		}
	}
	for &t in rest {
		d.string(t)?;
	}
	Ok(())
}

fn monsnote(d: &mut Disc) -> Res<()> {
	let texty = |b: &[u8]| !b.is_empty() && b.iter().all(|&c| c >= 0x20 || c == b'\n');
	let mut o = 0;
	loop {
		if d.u32(o)? == 0xFFFF_FFFF {
			d.data(o, d.len() - o)?;
			return Ok(());
		}
		let end = o + 8 + d.u32(o + 4)?;
		let rec = d.bytes(o, end)?;
		let last_nul = |upto: usize| rec[..upto].iter().rposition(|&c| c == 0);
		let desc = rec.len().checked_sub(1).filter(|&l| rec[l] == 0).and_then(last_nul);
		let name = desc.and_then(last_nul);
		match (name, desc) {
			(Some(n), Some(t)) if texty(&rec[n + 1..t]) && texty(&rec[t + 1..rec.len() - 1]) => {
				d.data(o, n + 1)?;
				d.string(o + n + 1)?;
				d.string(o + t + 1)?;
			}
			_ => {
				d.data(o, end - o)?;
			}
		}
		d.size32(o + 4, end);
		o = end;
	}
}

fn mstqrt(d: &mut Disc) -> Res<()> {
	d.data(0, 110 * 20)?;
	for t in d.ptr_table(110 * 20, usize::MAX)? {
		d.string(t)?;
	}
	Ok(())
}

fn town(d: &mut Disc) -> Res<()> {
	let n = d.u16(0)? / 2;
	d.data(0, n * 2)?;
	let b = d.p16(0)?;
	let nb = (d.u16(2)?.saturating_sub(b)) / 2;
	d.data(b, nb * 2)?;
	for i in 1..n {
		let t = d.p16(2 * i)?;
		d.string(t)?;
	}
	for i in 0..nb {
		let t = d.p16(b + 2 * i)?;
		d.string(t)?;
	}
	Ok(())
}

fn name(d: &mut Disc) -> Res<()> {
	let mut o = 0;
	loop {
		d.data(o, 20)?;
		let id = d.u16(o)?;
		d.str16(o + 2)?;
		o += 20;
		if id == 999 { break }
		if o + 20 > d.len() { return Err(tr("не найдена завершающая запись (id 999)", "the final record (id 999) was not found").into()) }
	}
	Ok(())
}

fn ittxt(d: &mut Disc) -> Res<()> {
	let n = d.u16(0)? / 2;
	d.data(0, n * 2)?;
	let mut tops = Vec::new();
	for i in 0..n {
		tops.push(d.p16(2 * i)?);
	}
	let Some(&first) = tops.first() else { return Err(tr("пустая таблица категорий", "empty category table").into()) };
	let mut pos = first;
	let mut min_rec = usize::MAX;
	let mut slots = Vec::new();
	while pos < min_rec {
		let v = d.u16(pos)?;
		min_rec = min_rec.min(v);
		slots.push(pos);
		pos += 2;
	}
	let mut bounds = tops.clone();
	bounds.sort();
	bounds.dedup();
	for (i, &t) in bounds.iter().enumerate() {
		let e = bounds.get(i + 1).copied().unwrap_or(pos);
		if e > t { d.data(t, e - t)?; }
	}
	for sl in slots {
		let r = d.p16(sl)?;
		d.data(r, 8)?;
		d.str16(r + 4)?;
		d.str16(r + 6)?;
	}
	Ok(())
}

fn quest(d: &mut Disc) -> Res<()> {
	let mut o = 0;
	let mut end = usize::MAX;
	let mut lists = Vec::new();
	while o < end {
		d.data(o, 28)?;
		for k in [12, 16, 20] {
			let t = d.p32(o + k)?;
			end = end.min(t);
			d.string(t)?;
		}
		let l = d.u32(o + 24)?;
		end = end.min(l);
		lists.push((o + 24, l));
		o += 28;
	}
	let starts: std::collections::HashSet<usize> = lists.iter().map(|a| a.1).collect();
	let mut done = std::collections::HashSet::new();
	for (slot, l) in lists {
		d.p32(slot)?;
		if !done.insert(l) { continue }
		let mut p = l;
		while p + 4 <= d.len() && (p == l || !starts.contains(&p)) {
			let v = d.u32(p)?;
			if v >= d.len() || v == 0 { break }
			p += 4;
		}
		d.data(l, p - l)?;
		let mut q = l;
		while q < p {
			let t = d.p32(q)?;
			d.string(t)?;
			q += 4;
		}
	}
	Ok(())
}

fn split_records(d: &mut Disc, targets: &[usize], end: usize) -> Res<()> {
	let mut ts: Vec<usize> = targets.to_vec();
	ts.sort();
	ts.dedup();
	ts.push(end);
	for w in ts.windows(2) {
		d.data(w[0], span(w[0], w[1])?)?;
	}
	Ok(())
}

fn fish(d: &mut Disc) -> Res<()> {
	let mut s = Vec::new();
	for i in 0..6 {
		s.push(d.p16(2 * i)?);
	}
	d.data(0, 12)?;
	let mut desc_min = usize::MAX;
	let mut o = s[0];
	while o < s[1] {
		d.data(o, 60)?;
		if let Some(t) = d.str16(o + 2)? { desc_min = desc_min.min(t) }
		o += 60;
	}
	d.data(s[1], span(s[1], s[2])?)?;
	d.data(s[2], span(s[2], desc_min.min(s[3]))?)?;
	for t in d.ptr_table(s[3], usize::MAX)? { d.string(t)?; }
	let recs = d.ptr_table(s[4], usize::MAX)?;
	split_records(d, &recs, s[5])?;
	for t in d.ptr_table(s[5], usize::MAX)? { d.string(t)?; }
	Ok(())
}

fn fixed_records(d: &mut Disc, size: usize, ptrs: &[usize]) -> Res<()> {
	let mut o = 0;
	let mut end = usize::MAX;
	while o < end {
		d.data(o, size)?;
		for &k in ptrs {
			if let Some(t) = d.str16(o + k)? { end = end.min(t) }
		}
		o += size;
		if o >= d.len() { return Err(tr("записи не заканчиваются", "the records do not end").into()) }
	}
	Ok(())
}

fn record(d: &mut Disc) -> Res<()> { fixed_records(d, 12, &[8, 10]) }
fn exmov(d: &mut Disc) -> Res<()> { fixed_records(d, 8, &[0, 2]) }

fn extra16(d: &mut Disc) -> Res<()> {
	let mut o = 0;
	loop {
		if d.bytes(o, o + 8)?.iter().all(|&b| b == 0) {
			d.data(o, 12)?;
			d.str16(o + 8)?;
			d.str16(o + 10)?;
			break;
		}
		d.data(o, 16)?;
		d.str16(o + 12)?;
		d.str16(o + 14)?;
		o += 16;
	}
	Ok(())
}

fn magic(d: &mut Disc) -> Res<()> {
	let ts = d.ptr_table(0, usize::MAX)?;
	let mut u = ts.clone();
	u.sort();
	u.dedup();
	let mut cnt = std::collections::BTreeMap::new();
	for w in u.windows(2) {
		let sz = w[1] - w[0];
		if sz > 4 {
			*cnt.entry(sz).or_insert(0usize) += 1;
		}
	}
	let full = cnt.iter().max_by_key(|(_, c)| **c).map(|(s, _)| *s).unwrap_or(28);
	let mut smin = usize::MAX;
	for w in u.windows(2) {
		if w[1] - w[0] == full {
			for k in [full - 4, full - 2] {
				let v = d.u16(w[0] + k)?;
				if v != 0 {
					smin = smin.min(v);
				}
			}
		}
	}
	for (i, &a) in u.iter().enumerate() {
		if a >= smin {
			d.string(a)?;
			continue;
		}
		let size = match u.get(i + 1) { Some(&b) => (b - a).min(if b >= smin { full } else { b - a }), None => full };
		d.data(a, size)?;
		if size == full {
			d.str16(a + full - 4)?;
			d.str16(a + full - 2)?;
		}
	}
	Ok(())
}

fn either(d: &mut Disc, a: Rule, b: Rule) -> Res<()> {
	let mut first = Disc::new(d.d);
	match a(&mut first).and_then(|_| first.clone().finish().map(|_| ())) {
		Ok(()) => {
			*d = first;
			Ok(())
		}
		Err(ea) => {
			let mut second = Disc::new(d.d);
			match b(&mut second) {
				Ok(()) => {
					*d = second;
					Ok(())
				}
				Err(eb) => Err(lformat!("{ea} (вариант Zero: {eb})", "{ea} (Zero variant: {eb})")),
			}
		}
	}
}

fn fish_any(d: &mut Disc) -> Res<()> {
	either(d, fish, fish_zero)
}
fn extra_any(d: &mut Disc) -> Res<()> {
	either(d, extra16, extra12)
}
fn cook_any(d: &mut Disc) -> Res<()> {
	either(d, cook, cook_zero)
}

fn fish_zero(d: &mut Disc) -> Res<()> {
	let s0 = d.p16(0)?;
	let s1 = d.p16(2)?;
	let s2 = d.p16(4)?;
	d.data(0, 6)?;
	if s0 != 6 || s1 <= s0 || s2 <= s1 {
		return Err(tr("не похоже на t_fish Zero", "does not look like Zero's t_fish").into());
	}
	let mut desc_min = usize::MAX;
	let mut o = s0;
	while o + 60 <= s1 {
		d.data(o, 60)?;
		if let Some(t) = d.str16(o + 2)? {
			desc_min = desc_min.min(t);
		}
		o += 60;
	}
	d.data(o, s1 - o)?;
	d.data(s1, s2 - s1)?;
	if desc_min != usize::MAX && desc_min > s2 {
		d.data(s2, desc_min - s2)?;
	} else {
		d.data_open(s2)?;
	}
	Ok(())
}

fn extra12(d: &mut Disc) -> Res<()> {
	let mut o = 0;
	loop {
		if d.bytes(o, o + 6)?.iter().all(|&b| b == 0) {
			d.data(o, 10)?;
			d.str16(o + 6)?;
			d.str16(o + 8)?;
			break;
		}
		d.data(o, 12)?;
		d.str16(o + 8)?;
		d.str16(o + 10)?;
		o += 12;
	}
	Ok(())
}

fn cook_zero(d: &mut Disc) -> Res<()> {
	let s0 = d.p16(0)?;
	let s1 = d.p16(2)?;
	d.data(0, 4)?;
	if s0 != 4 || s1 <= s0 {
		return Err(tr("не похоже на t_cook Zero", "does not look like Zero's t_cook").into());
	}
	let mut smin = usize::MAX;
	let mut o = s0;
	while o + 48 <= s1 {
		d.data(o, 48)?;
		if let Some(t) = d.str16(o + 44)? {
			smin = smin.min(t);
		}
		o += 48;
	}
	if o < s1 {
		d.data(o, s1 - o)?;
	}
	if smin != usize::MAX && smin > s1 {
		d.data(s1, smin - s1)?;
	} else {
		d.data_open(s1)?;
	}
	Ok(())
}

fn cook(d: &mut Disc) -> Res<()> {
	let mut s = Vec::new();
	for i in 0..3 {
		s.push(d.p16(2 * i)?);
	}
	d.data(0, 6)?;
	let mut smin = usize::MAX;
	let mut o = s[0];
	while o + 60 <= s[1] {
		d.data(o, 60)?;
		if let Some(t) = d.str16(o + 58)? { smin = smin.min(t) }
		o += 60;
	}
	d.data(s[1], span(s[1], s[2])?)?;
	if smin != usize::MAX && smin > s[2] {
		d.data(s[2], smin - s[2])?;
	} else {
		d.data_open(s[2])?;
	}
	Ok(())
}

fn shop(d: &mut Disc) -> Res<()> {
	let mut ts = d.ptr_table(0, usize::MAX)?;
	ts.sort();
	ts.dedup();
	for t in ts {
		d.data(t, 16)?;
		let items = d.p16(t + 12)?;
		d.data_open(items)?;
		d.str16(t + 14)?;
	}
	Ok(())
}

fn mgame(d: &mut Disc) -> Res<()> {
	let ts = d.ptr_table(0, usize::MAX)?;
	let mut u = ts.clone();
	u.sort();
	u.dedup();
	u.push(d.len());
	for w in u.windows(2) {
		let (a, b) = (w[0], w[1]);
		let r = d.bytes(a, b)?.to_vec();
		let texty = |x: &[u8]| !x.is_empty() && x.ends_with(&[0]) && x.iter().filter(|&&c| c >= 0x20).count() * 2 > x.len();
		if r.len() > 11 && r[0] == 0xFF && r[1] == 0xFF && texty(&r[11..]) {
			d.data(a, 11)?;
			d.strings_until(a + 11, b)?;
		} else if texty(&r) {
			d.strings_until(a, b)?;
		} else {
			d.data(a, b - a)?;
		}
	}
	Ok(())
}
