//! Asset directory for the game binary: env, packaged layout, then crate `assets/`.

use std::env;
use std::path::{Path, PathBuf};

/// Bevy [`bevy::asset::AssetPlugin`] file root.
///
/// Order: `MAYBRAID_ASSETS`, then a macOS bundle or sidecar `assets/` next to
/// the executable, then this crate's `assets/` symlink (`maybraid/assets`).
pub fn assets_root() -> PathBuf {
	if let Ok(path) = env::var("MAYBRAID_ASSETS") {
		return PathBuf::from(path);
	}
	if let Ok(exe) = env::current_exe() {
		if let Some(root) = assets_beside_executable(&exe) {
			return root;
		}
	}
	Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// Packaged layouts: `Contents/Resources/assets` or `assets/` beside the exe.
pub fn assets_beside_executable(exe: &Path) -> Option<PathBuf> {
	let exe_dir = exe.parent()?;
	if let Some(assets) = macos_bundle_assets(exe_dir) {
		if assets.is_dir() {
			return Some(assets);
		}
	}
	let sidecar = exe_dir.join("assets");
	sidecar.is_dir().then_some(sidecar)
}

fn macos_bundle_assets(exe_dir: &Path) -> Option<PathBuf> {
	if exe_dir.file_name()? != "MacOS" {
		return None;
	}
	let contents = exe_dir.parent()?;
	if contents.file_name()? != "Contents" {
		return None;
	}
	Some(contents.join("Resources").join("assets"))
}

#[cfg(test)]
mod tests {
	use super::{assets_beside_executable, macos_bundle_assets};
	use std::fs;
	use std::path::Path;

	#[test]
	fn bundle_layout_points_at_resources_assets() {
		let macos = Path::new("/Applications/Maybraid.app/Contents/MacOS");
		assert_eq!(
			macos_bundle_assets(macos).as_deref(),
			Some(Path::new("/Applications/Maybraid.app/Contents/Resources/assets"))
		);
		assert!(macos_bundle_assets(Path::new("/tmp/target/release")).is_none());
	}

	#[test]
	fn sidecar_assets_win_when_the_directory_exists() {
		let dir = tempfile::tempdir().expect("tempdir");
		let exe = dir.path().join("maybraid");
		fs::write(&exe, []).expect("exe");
		assert!(assets_beside_executable(&exe).is_none());
		fs::create_dir(dir.path().join("assets")).expect("assets");
		assert_eq!(
			assets_beside_executable(&exe).as_deref(),
			Some(dir.path().join("assets").as_path())
		);
	}
}
