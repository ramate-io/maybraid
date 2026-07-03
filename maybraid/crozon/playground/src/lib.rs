//! Interactive viewer for Crozon modular character rigs.

mod animation;
pub mod camera;
pub mod character;
pub mod checkerboard_material;
pub mod commands;
mod ground;
pub mod skinning;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use animation::{animate_limbs, init_limb_animators, AnimationArticulationDebug};
use bevy::prelude::*;
use camera_controls::look::CameraLookPlugin;
use character::CharacterConfig;
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use ground::setup_ground;
use skinning::{
	attach_parts_to_sockets, build_rig_bone_map, dump_bones_to_console, remap_part_skin_to_rig,
	DumpBonesRequest,
};

pub struct CrozonCharacterPlaygroundPlugin;

impl Plugin for CrozonCharacterPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<CharacterConfig>()
			.init_resource::<character::CharacterSyncState>()
			.init_resource::<AnimationArticulationDebug>()
			.init_resource::<DumpBonesRequest>()
			.add_plugins(CameraLookPlugin::default())
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
			.add_plugins(
				bevy::pbr::MaterialPlugin::<checkerboard_material::CheckerboardMaterial>::default(),
			)
			.add_systems(Startup, (camera::setup_camera, setup_lighting, setup_ground))
			.add_systems(
				Update,
				(
					camera::camera_controller,
					character::sync_character
						.after(capture_command_line_input::<PlaygroundCommand>),
					build_rig_bone_map,
					attach_parts_to_sockets.after(build_rig_bone_map),
					remap_part_skin_to_rig.after(attach_parts_to_sockets),
					init_limb_animators.after(build_rig_bone_map),
					animate_limbs.after(init_limb_animators),
					dump_bones_to_console,
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
