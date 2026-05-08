//! Interactive viewer for [`sdf_common`] primitives via marching-cubes meshing.
//!
//! Commands are typed in-game after **`/`** (see [`commands::PlaygroundCommand::parse_line`]).

pub mod camera;
pub mod checkerboard_material;
pub mod commands;
mod ground;
mod input;
mod preview;
pub mod primitive;
mod ui;

pub use camera::CameraController;
pub use commands::PlaygroundCommand;
pub use ground::PlaygroundSettings;
pub use preview::{PreviewConfig, SdfPreviewRoot};

use bevy::prelude::*;
use ground::setup_ground;
use commands::react::{react_playground_command_root, react_render_to_preview, react_settings_to_playground};
use preview::{keyboard_preview, sync_sdf_preview};
use render_item::{mesh::fetch_meshes, mesh::handle::MeshHandle, render_items};
use primitive::{PlaygroundPrimitive, PlaygroundRenderItem};

/// Brown-ish default material for SDF previews (similar stick/trunk tone in objects playground).
#[derive(Resource, Clone)]
pub struct PlaygroundMaterial(pub Handle<StandardMaterial>);

pub struct SdfCommonPlaygroundPlugin;

impl Plugin for SdfCommonPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PlaygroundSettings>()
			.init_resource::<PreviewConfig>()
			.add_plugins(bevy::pbr::MaterialPlugin::<checkerboard_material::CheckerboardMaterial>::default())
			.init_resource::<input::TypedCommandLine>()
			.init_resource::<input::TextEntryFocus>()
			.init_resource::<input::CommandConsoleOutput>()
			.add_systems(
				Startup,
				(
					camera::setup_camera,
					setup_lighting,
					setup_ground,
					setup_preview_material,
					ui::setup_debug_ui,
				),
			)
			.add_systems(
				Update,
				(
					camera::camera_controller,
					keyboard_preview,
					input::toggle_text_entry_focus,
					input::capture_command_line_input,
					react_playground_command_root,
					react_render_to_preview,
					react_settings_to_playground,
					sync_sdf_preview,
					ui::update_debug_ui,
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
