//! Kit-item viewer: socketed firearm concepts on the shared receiver rig.

mod camera;
pub mod commands;
mod preview;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use camera_controls::look::{CameraLookConfig, CameraLookPlugin};
use firearms::FirearmHostsPlugin;
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use preview::{sync_preview, PreviewConfig};

pub struct ItemsPlaygroundPlugin;

impl Plugin for ItemsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PreviewConfig>()
			.add_plugins(CameraLookPlugin::new(CameraLookConfig {
				enabled_at_start: false,
				..CameraLookConfig::default()
			}))
			.add_plugins(FirearmHostsPlugin)
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
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
	commands.spawn((
		DirectionalLight { illuminance: 10000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}

fn setup_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let mesh = meshes.add(Plane3d::default().mesh().size(20.0, 20.0));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.78, 0.8, 0.76),
		perceptual_roughness: 0.9,
		..default()
	});
	commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::IDENTITY));
}
