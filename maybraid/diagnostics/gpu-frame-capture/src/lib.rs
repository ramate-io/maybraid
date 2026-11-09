//! Opt-in, one-render-update Metal diagnostics. Never enable in benchmark builds.

use bevy::prelude::*;

#[cfg(target_os = "macos")]
mod metal;

pub struct GpuFrameCapturePlugin;

impl Plugin for GpuFrameCapturePlugin {
	fn build(&self, _app: &mut App) {}

	fn finish(&self, app: &mut App) {
		#[cfg(target_os = "macos")]
		metal::install(app);
		#[cfg(not(target_os = "macos"))]
		{
			let _ = app;
			if std::env::var_os("MAYBRAID_GPU_CAPTURE_OUTPUT").is_some() {
				eprintln!("[gpu-frame-capture] Metal capture requires macOS");
			}
		}
	}
}
