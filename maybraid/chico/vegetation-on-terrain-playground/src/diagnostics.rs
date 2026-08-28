//! Frame timing diagnostics for the vegetation-on-terrain playground.
//!
//! Toggle with env `CHICO_VEG_TERRAIN_DIAG` (comma-separated):
//! - `fps` — throttled `[veg.timing]` FPS / frame_ms plus an on-screen HUD
//! - `off` — disable (default when unset)
//!
//! The world playground inserts [`PlaygroundDiag`] `{ fps: true }` before this
//! plugin so the HUD stays on without the env flag.
//!
//! Examples:
//! ```text
//! CHICO_VEG_TERRAIN_DIAG=fps
//! CHICO_VEG_TERRAIN_DIAG=off   # default
//! ```

use std::time::Duration;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

const ENV_DIAG: &str = "CHICO_VEG_TERRAIN_DIAG";
const LOG_INTERVAL: Duration = Duration::from_secs(1);

/// Parsed [`CHICO_VEG_TERRAIN_DIAG`] flags.
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
			return Self { fps: false };
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

#[derive(Component)]
struct FrameHudRoot;

#[derive(Component)]
struct FrameHudText;

/// Toggle FPS log + HUD (`/stats fps` in the world playground).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestFpsToggle;

pub struct PlaygroundTimingPlugin;

impl Plugin for PlaygroundTimingPlugin {
	fn build(&self, app: &mut App) {
		if !app.world().contains_resource::<PlaygroundDiag>() {
			app.insert_resource(PlaygroundDiag::from_env());
		}
		if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
			app.add_plugins(FrameTimeDiagnosticsPlugin::default());
		}
		app.add_systems(Startup, spawn_frame_hud)
			.add_systems(Update, (toggle_fps_logging, log_frame_timing, update_frame_hud));
	}
}

pub fn toggle_fps_logging(
	mut commands: Commands,
	mut diag: ResMut<PlaygroundDiag>,
	mut status: ResMut<game_commands::ui::GameCommandStatusText>,
	requests: Query<Entity, With<RequestFpsToggle>>,
) {
	for entity in &requests {
		diag.fps = !diag.fps;
		status.0 =
			if diag.fps { "[veg.timing] fps on".into() } else { "[veg.timing] fps off".into() };
		info!("{}", status.0);
		commands.entity(entity).despawn();
	}
}

fn spawn_frame_hud(mut commands: Commands) {
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(8.0),
				right: Val::Px(8.0),
				padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
				..default()
			},
			BackgroundColor(Color::srgba(0.05, 0.08, 0.12, 0.78)),
			Visibility::Hidden,
			FrameHudRoot,
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new("fps —"),
				TextFont { font_size: bevy::text::FontSize::Px(14.0), ..default() },
				TextColor(Color::srgb(0.92, 0.96, 1.0)),
				FrameHudText,
			));
		});
}

fn update_frame_hud(
	diag: Res<PlaygroundDiag>,
	diagnostics: Res<DiagnosticsStore>,
	mut root: Query<&mut Visibility, With<FrameHudRoot>>,
	mut text: Query<&mut Text, With<FrameHudText>>,
) {
	let Ok(mut visibility) = root.single_mut() else {
		return;
	};
	if !diag.fps {
		*visibility = Visibility::Hidden;
		return;
	}
	*visibility = Visibility::Visible;
	let Ok(mut hud) = text.single_mut() else {
		return;
	};
	let fps = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FPS)
		.and_then(|d| d.smoothed())
		.unwrap_or(f64::NAN);
	let frame_ms = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
		.and_then(|d| d.smoothed())
		.unwrap_or(f64::NAN);
	*hud = Text::new(format!("fps {fps:.0}   {frame_ms:.1} ms"));
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
	eprintln!("[veg.timing] fps={fps:.1} frame_ms={frame_ms:.2}");
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn summary_names_fps_flag() {
		assert!(PlaygroundDiag { fps: true }.summary().contains("fps"));
		assert!(PlaygroundDiag { fps: false }.summary().contains("off"));
	}
}
