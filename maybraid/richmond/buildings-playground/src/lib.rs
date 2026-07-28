//! Interactive viewer for Richmond building components and authored buildings.

pub mod camera;
pub mod commands;
mod ground;
mod preview;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use preview::{PreviewConfig, PreviewSubject};

use bevy::prelude::*;
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use ground::setup_ground;
use mesh_ref::MeshRefPlugin;
use preview::sync_preview;

pub struct RichmondBuildingsPlaygroundPlugin;

impl Plugin for RichmondBuildingsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PreviewConfig>()
			.add_plugins((
				MeshRefPlugin,
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()),
			))
			.add_systems(Startup, (camera::setup_camera, setup_lighting, setup_ground))
			.add_systems(
				Update,
				(
					camera::camera_controller,
					sync_preview.after(capture_command_line_input::<PlaygroundCommand>),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	// Key light (casts shadows).
	commands.spawn((
		DirectionalLight {
			illuminance: 10000.0,
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
	));
	// Fill from the opposite side (no shadows) to soften contrast.
	commands.spawn((
		DirectionalLight {
			illuminance: 3500.0,
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 5.0, -PI / 4.0, 0.0)),
	));
	// Soft bounce / skylight fill.
	commands.spawn((
		DirectionalLight {
			illuminance: 1800.0,
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 6.0, PI + PI / 3.0, 0.0)),
	));
}
