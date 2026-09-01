//! Firing range: kit guns auto-fire emissive bolts, bullets, and lasers.

mod camera;
pub mod commands;
mod range;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use camera_controls::look::{CameraLookConfig, CameraLookPlugin};
use firearms::{FirearmHostsPlugin, FirearmWeaponsPlugin};
use game_commands::command::GameCommandPlugin;

pub struct FiringRangePlugin;

impl Plugin for FiringRangePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(CameraLookPlugin::new(CameraLookConfig {
			enabled_at_start: false,
			..CameraLookConfig::default()
		}))
		.add_plugins(FirearmHostsPlugin)
		.add_plugins(FirearmWeaponsPlugin)
		.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
		.add_systems(Startup, (camera::setup_camera, setup_lighting, range::setup_range))
		.add_systems(
			Update,
			(
				camera::release_modifiers_on_focus_change.before(camera::camera_controller),
				camera::camera_controller,
				ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
			),
		);
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.spawn((
		DirectionalLight { illuminance: 2500.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 200.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 3.0, 0.0)),
	));
}
