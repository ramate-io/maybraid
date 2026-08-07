//! Playground CPU / GPU timing diagnostics.
//!
//! - [`FrameTimeDiagnosticsPlugin`] + throttled `[veg.timing]` lines for FPS / frame time
//! - Top Bevy [`RenderDiagnosticsPlugin`] paths (`render/.../elapsed_gpu|elapsed_cpu`) when present
//!
//! Command / archetype apply is timed separately in `main` via Bevy's `system_commands`
//! spans (`[lod.commands]`; requires the `trace` feature).

use std::time::Duration;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

const LOG_INTERVAL: Duration = Duration::from_secs(1);
const TOP_RENDER_PATHS: usize = 8;
/// Skip tiny render spans so the log stays readable.
const RENDER_MIN_MS: f64 = 0.25;

pub struct PlaygroundTimingPlugin;

impl Plugin for PlaygroundTimingPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
			app.add_plugins(FrameTimeDiagnosticsPlugin::default());
		}
		app.add_systems(Update, log_frame_and_render_timing);
	}
}

fn log_frame_and_render_timing(
	time: Res<Time>,
	mut accum: Local<Duration>,
	diagnostics: Res<DiagnosticsStore>,
) {
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

	// eprintln avoids re-entrancy through the logging subscriber (same as [lod.commands]).
	eprintln!("[veg.timing] fps={fps:.1} frame_ms={frame_ms:.2}");

	let mut render: Vec<(&str, f64)> = diagnostics
		.iter()
		.filter_map(|d| {
			let path = d.path().as_str();
			if !(path.contains("elapsed_gpu") || path.contains("elapsed_cpu")) {
				return None;
			}
			let value = d.smoothed().or_else(|| d.value())?;
			(value >= RENDER_MIN_MS).then_some((path, value))
		})
		.collect();
	render.sort_by(|a, b| b.1.total_cmp(&a.1));
	for (path, ms) in render.into_iter().take(TOP_RENDER_PATHS) {
		eprintln!("[veg.render] {path}={ms:.2}ms");
	}
}
