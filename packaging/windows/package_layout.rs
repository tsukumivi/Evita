use std::{fs, path::Path};

/// Top-level files and directories accepted in a Windows portable package.
/// Keep this list shared by the packager and the in-app updater.
pub const WINDOWS_FILES: &[&str] = &[
	"evita.exe",
	"README.md",
	"LICENSE-MIT",
	"LICENSE-APACHE",
	"THIRD_PARTY_NOTICES.md",
	"docs",
	"licenses",
	"source",
	"install-notifications.ps1",
	"setup.ps1",
	"WebView2Loader.dll",
];

/// Cargo's GNU Windows toolchain may leave a hash-suffixed executable name in
/// the main executable's import table. Only accept a bounded hexadecimal hash.
pub fn is_self_import_alias(name: &str) -> bool {
	let Some(hash) = name
		.strip_prefix("evita-")
		.and_then(|name| name.strip_suffix(".exe"))
	else {
		return false;
	};
	(8..=64).contains(&hash.len()) && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn is_windows_package_file(name: &str) -> bool {
	WINDOWS_FILES.contains(&name) || is_self_import_alias(name)
}

/// Return and validate the actual top-level entries that the update helper must install.
/// This is used by the Windows updater helper and xtask's packager; other desktop targets
/// still include this module for the shared Windows ZIP allow-list.
#[allow(dead_code)]
pub fn windows_package_files(root: &Path) -> Result<Vec<String>, String> {
	let entries =
		fs::read_dir(root).map_err(|error| format!("Cannot read Windows package: {error}"))?;
	let mut files = Vec::new();
	let mut has_import_alias = false;
	for entry in entries {
		let entry = entry.map_err(|error| format!("Cannot inspect Windows package: {error}"))?;
		let name = entry
			.file_name()
			.into_string()
			.map_err(|_| "Windows package contains a non-UTF-8 filename.".to_owned())?;
		let kind = entry
			.file_type()
			.map_err(|error| format!("Cannot inspect Windows package entry: {error}"))?;
		if kind.is_symlink() || (!kind.is_file() && !kind.is_dir()) {
			return Err(format!("Windows package contains an unsafe entry: {name}"));
		}
		if is_self_import_alias(&name) {
			if !kind.is_file() || has_import_alias {
				return Err(format!(
					"Windows package contains an invalid import alias: {name}"
				));
			}
			has_import_alias = true;
		}
		if !is_windows_package_file(&name) {
			return Err(format!(
				"Windows package contains unexpected content: {name}"
			));
		}
		files.push(name);
	}
	files.sort_unstable();
	Ok(files)
}
