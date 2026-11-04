use std::path::{Path, PathBuf};

fn main() {
	if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
		use embed_manifest::{embed_manifest, new_manifest, manifest::DpiAwareness};
		embed_manifest(new_manifest("CrossbellTool").dpi_awareness(DpiAwareness::PerMonitorV2))
			.expect("unable to embed manifest");
		let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
		let icons = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/icons");
		for (ico, bin) in [("crossbell_tool.ico", "CrossbellTool"), ("crossbell_compare.ico", "CrossbellCompare")] {
			let stem = ico.trim_end_matches(".ico");
			match icon_rc(&out, &icons.join(ico), stem) {
				Some(obj) => println!("cargo:rustc-link-arg-bin={bin}={}", obj.display()),
				None => println!("cargo:warning=rc.exe (Windows SDK) not found, icon {ico} was not added"),
			}
		}
	}
	versions();
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=src/icons");
}

fn versions() {
	let path = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("versions.txt");
	println!("cargo:rerun-if-changed={}", path.display());
	let text = std::fs::read_to_string(&path).expect("versions.txt is missing next to Cargo.toml");
	let mut suffix = String::new();
	for l in text.lines() {
		let l = l.trim();
		let Some((k, v)) = l.split_once('=') else { continue };
		let (k, v) = (k.trim(), v.trim());
		if k.eq_ignore_ascii_case("suffix") {
			if !v.is_empty() {
				suffix = format!(" {v}");
			}
			continue;
		}
		let name: String = k.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_uppercase();
		println!("cargo:rustc-env=VER_{name}={v}");
	}
	println!("cargo:rustc-env=VER_SUFFIX={suffix}");
}

fn icon_rc(out: &Path, ico: &Path, stem: &str) -> Option<PathBuf> {
	let rc = out.join(format!("{stem}_icon.rc"));
	let res = out.join(format!("{stem}_icon.res"));
	let text = format!("1 ICON \"{}\"\r\n", ico.display().to_string().replace('\\', "/"));
	let mut bytes = vec![0xFF, 0xFE];
	for u in text.encode_utf16() {
		bytes.extend_from_slice(&u.to_le_bytes());
	}
	std::fs::write(&rc, bytes).ok()?;
	let run = |exe: &Path| std::process::Command::new(exe).arg("/nologo").arg("/fo").arg(&res).arg(&rc).status().map(|s| s.success()).unwrap_or(false);
	if run(Path::new("rc.exe")) || find_sdk_rc().map(|p| run(&p)).unwrap_or(false) {
		Some(res)
	} else {
		None
	}
}

fn find_sdk_rc() -> Option<PathBuf> {
	let mut roots: Vec<PathBuf> = Vec::new();
	if let Some(d) = std::env::var_os("WindowsSdkDir") {
		roots.push(PathBuf::from(d).join("bin"));
	}
	for pf in ["ProgramFiles(x86)", "ProgramFiles"] {
		if let Some(d) = std::env::var_os(pf) {
			roots.push(PathBuf::from(d).join("Windows Kits").join("10").join("bin"));
		}
	}
	for root in roots {
		let Ok(rd) = std::fs::read_dir(&root) else { continue };
		let mut vers: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.join("x64").join("rc.exe").is_file()).collect();
		vers.sort();
		if let Some(v) = vers.pop() {
			return Some(v.join("x64").join("rc.exe"));
		}
	}
	None
}
