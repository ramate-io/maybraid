//! Throttled FPS and frame-time logging for the firing range terminal.

use std::time::Duration;

use bevy::{
	diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
	platform::collections::HashSet,
	prelude::*,
};

const LOG_INTERVAL_SECS: f32 = 2.0;

pub(crate) struct FiringRangeDiagnosticsPlugin;

impl Plugin for FiringRangeDiagnosticsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
			app.add_plugins(FrameTimeDiagnosticsPlugin::default());
		}
		if !app.is_plugin_added::<LogDiagnosticsPlugin>() {
			app.add_plugins(LogDiagnosticsPlugin {
				wait_duration: Duration::from_secs_f32(LOG_INTERVAL_SECS),
				filter: Some(HashSet::from_iter([
					FrameTimeDiagnosticsPlugin::FPS,
					FrameTimeDiagnosticsPlugin::FRAME_TIME,
				])),
				debug: false,
			});
		}
	}
}
