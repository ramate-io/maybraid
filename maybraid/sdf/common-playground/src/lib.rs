//! Interactive viewer for [`sdf_common`] primitives via marching-cubes meshing.
//!
//! Commands are typed in-game after **`/`** (see [`commands::PlaygroundCommand::parse_line`]) or passed on the process command line (see [`commands::PlaygroundCommand::parse_startup_command`] and [`PendingStartupCommand`]).

pub mod camera;
pub mod checkerboard_material;
pub mod commands;
mod ground;
mod preview;
pub mod primitive;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PlaygroundCommandsPlugin, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use ground::PlaygroundSettings;
pub use preview::{PreviewConfig, SdfPreviewRoot};

use bevy::prelude::*;
use commands::settings::react_settings_announcer::despawn_settings_command_announcer;
use game_commands::command::GameCommandPlugin;
use ground::setup_ground;
use preview::{keyboard_preview, sync_sdf_preview};
use primitive::{PlaygroundPrimitive, PlaygroundRenderItem};
use render_item::{mesh::fetch_meshes, mesh::handle::MeshHandle, render_items};

/// Brown-ish default material for SDF previews (similar stick/trunk tone in objects playground).
#[derive(Resource, Clone)]
pub struct PlaygroundMaterial(pub Handle<StandardMaterial>);

pub struct SdfCommonPlaygroundPlugin;

impl Plugin for SdfCommonPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PlaygroundSettings>()
			.init_resource::<PreviewConfig>()
			.add_plugins(PlaygroundCommandsPlugin)
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
			.add_plugins(
				bevy::pbr::MaterialPlugin::<checkerboard_material::CheckerboardMaterial>::default(),
			)
			.add_systems(
				Startup,
				(camera::setup_camera, setup_lighting, setup_ground, setup_preview_material),
			)
			.add_systems(
				Update,
				(
					camera::camera_controller,
					keyboard_preview,
					sync_sdf_preview.after(despawn_settings_command_announcer),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
					render_items::<PlaygroundRenderItem<StandardMaterial>>,
					fetch_meshes::<MeshHandle<PlaygroundPrimitive>, StandardMaterial>,
				),
			);
	}
}

fn setup_preview_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
	let handle = materials
		.add(StandardMaterial { base_color: Color::srgb(0.89, 0.886, 0.604), ..default() });
	commands.insert_resource(PlaygroundMaterial(handle));
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.spawn((
		DirectionalLight { illuminance: 10000.0, shadows_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 500.0, shadows_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}
