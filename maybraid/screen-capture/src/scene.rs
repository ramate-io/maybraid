//! Complete-scene screenshot helpers.
//!
//! Window / HUD evidence uses this path. Isolated icon subjects stay on
//! [`crate::ScreenCapturePlugin`] and must not be forced through it.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};

/// Output directory and window-screenshot helpers for a full playground scene.
#[derive(Resource, Clone, Debug)]
pub struct SceneCapture {
	pub dir: PathBuf,
}

impl SceneCapture {
	pub fn new(dir: impl Into<PathBuf>) -> Self {
		Self { dir: dir.into() }
	}

	pub fn prepare(&self) -> std::io::Result<()> {
		std::fs::create_dir_all(&self.dir)
	}

	pub fn path(&self, file_name: impl AsRef<Path>) -> PathBuf {
		self.dir.join(file_name)
	}

	pub fn png_ready(&self, file_name: impl AsRef<Path>) -> bool {
		self.path(file_name).is_file()
	}

	/// GPU-readback of the primary window via Bevy `Screenshot`.
	pub fn request_window_png(
		&self,
		commands: &mut Commands,
		file_name: impl AsRef<Path>,
	) -> PathBuf {
		let path = self.path(file_name);
		commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
		path
	}

	/// ImageMagick `import -window` of a titled window (xvfb HUD captures).
	pub fn import_named_window(
		&self,
		window_title: &str,
		file_name: impl AsRef<Path>,
	) -> Result<PathBuf, String> {
		let out = self.path(file_name);
		let wid_bytes = std::process::Command::new("xdotool")
			.args(["search", "--name", window_title])
			.output()
			.map_err(|error| error.to_string())?;
		if !wid_bytes.status.success() {
			return Err(format!(
				"xdotool search failed: {}",
				String::from_utf8_lossy(&wid_bytes.stderr)
			));
		}
		let wid_text = String::from_utf8_lossy(&wid_bytes.stdout);
		let wid = wid_text.lines().next().unwrap_or("").trim();
		if wid.is_empty() {
			return Err("no window id".into());
		}
		let geom = std::process::Command::new("xwininfo")
			.args(["-id", wid])
			.output()
			.map_err(|error| error.to_string())?;
		let geom_text = String::from_utf8_lossy(&geom.stdout);
		let geometry = geom_text
			.lines()
			.find_map(|line| line.trim().strip_prefix("-geometry").map(|rest| rest.trim()))
			.ok_or("missing -geometry")?;
		let (w, h) = geometry
			.split_once('x')
			.and_then(|(w, rest)| rest.split_once('+').map(|(h, _)| (w, h)))
			.ok_or("bad geometry")?;
		let crop = format!("{w}x{h}+0+0");
		let out_path = out.to_string_lossy();
		let status = std::process::Command::new("import")
			.args(["-window", wid, "-crop", &crop, out_path.as_ref()])
			.status()
			.map_err(|error| error.to_string())?;
		if !status.success() {
			return Err("import failed".into());
		}
		Ok(out)
	}
}

/// True while a Bevy window screenshot is still encoding.
pub fn window_capture_in_flight(capturing: &Query<(), With<Capturing>>) -> bool {
	!capturing.is_empty()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn paths_keep_caller_file_names() -> anyhow::Result<()> {
		let capture = SceneCapture::new("/tmp/preview-capture");
		assert_eq!(
			capture.path("lubikan-hud-state-01-partial-above-health.png"),
			PathBuf::from("/tmp/preview-capture/lubikan-hud-state-01-partial-above-health.png")
		);
		assert_eq!(
			capture.path("01-drop-confirm.png"),
			PathBuf::from("/tmp/preview-capture/01-drop-confirm.png")
		);
		assert!(!capture.png_ready("missing.png"));
		Ok(())
	}
}
