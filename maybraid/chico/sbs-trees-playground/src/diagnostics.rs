//! Frame timing diagnostics for the SBS trees playground.
//!
//! Toggle with env `CHICO_SBS_DIAG` (comma-separated):
//! - `fps` — throttled `[sbs.timing]` FPS / frame_ms (default when unset)
//! - `off` — disable
//!
//! Examples:
//! ```text
//! CHICO_SBS_DIAG=fps   # default
//! CHICO_SBS_DIAG=off
//! ```

use std::time::Duration;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

const ENV_DIAG: &str = "CHICO_SBS_DIAG";
const LOG_INTERVAL: Duration = Duration::from_secs(1);

/// Parsed [`CHICO_SBS_DIAG`] flags.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaygroundDiag {
	pub fps: bool,
}

impl Default for PlaygroundDiag {
	fn default() -> Self {
		Self::from_env()
	}
}

impl PlaygroundDiag {
	pub fn from_env() -> Self {
		let raw = std::env::var(ENV_DIAG).unwrap_or_default();
		let raw = raw.trim();
		if raw.is_empty() {
			return Self { fps: true };
		}
		let mut fps = false;
		let mut off = false;
		for part in raw.split(',') {
			let part = part.trim();
			if part.is_empty() {
				continue;
			}
			match part.to_ascii_lowercase().as_str() {
				"off" | "none" | "0" => off = true,
				"fps" | "timing" | "on" => fps = true,
				other => {
					eprintln!("[{ENV_DIAG}] unknown flag {other:?} (use fps|off)");
				}
			}
		}
		if off {
			return Self { fps: false };
		}
		if !fps {
			fps = true;
		}
		Self { fps }
	}

	pub fn summary(self) -> String {
		if self.fps {
			format!("{ENV_DIAG}=fps")
		} else {
			format!("{ENV_DIAG}=off")
		}
	}
}

pub struct PlaygroundTimingPlugin;

impl Plugin for PlaygroundTimingPlugin {
	fn build(&self, app: &mut App) {
		let diag = PlaygroundDiag::from_env();
		app.insert_resource(diag);
		if diag.fps {
			if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
				app.add_plugins(FrameTimeDiagnosticsPlugin::default());
			}
			app.add_systems(Update, log_frame_timing);
		}
	}
}

fn log_frame_timing(
	time: Res<Time>,
	diag: Res<PlaygroundDiag>,
	mut accum: Local<Duration>,
	diagnostics: Res<DiagnosticsStore>,
) {
	if !diag.fps {
		return;
	}
	*accum += time.delta();
	if *accum < LOG_INTERVAL {
		return;
	}
	*accum = Duration::ZERO;

	let fps = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FPS)
		.and_then(|d| d.smoothed())
		.unwrap_or(f64::NAN);
	let frame_ms = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
		.and_then(|d| d.smoothed())
		.unwrap_or(f64::NAN);
	eprintln!("[sbs.timing] fps={fps:.1} frame_ms={frame_ms:.2}");
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_env_enables_fps() {
		assert!(PlaygroundDiag { fps: true }.summary().contains("fps"));
		assert!(PlaygroundDiag { fps: false }.summary().contains("off"));
	}
}
