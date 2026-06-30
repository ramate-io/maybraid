//! Command-driven playground for the Character Concepts Screen.
//!
//! This crate is the first executable surface for the concept plan. Commands are
//! temporary stand-ins for future UI fields; they still resolve through
//! `crozon-characters` before any Bevy entities are spawned.

mod animation;
pub mod commands;
mod ground;
mod preview;
mod skinning;
mod ui;

pub use commands::{ConceptsCommand, CONCEPTS_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use crozon_character_playground::{camera, checkerboard_material};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};

use animation::{animate_body_rig, init_limb_animators};
use preview::{sync_preview, ConceptPreviewConfig, ConceptPreviewSyncState};
use skinning::{
	attach_parts_to_sockets, build_rig_bone_map, dump_bones_to_console, maintain_resolved_pose,
	prune_duplicate_part_scenes, remap_part_skin_to_rig, DumpBonesRequest,
};

pub struct CrozonCharacterConceptsPlaygroundPlugin;

impl Plugin for CrozonCharacterConceptsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<ConceptPreviewConfig>()
			.init_resource::<ConceptPreviewSyncState>()
			.init_resource::<DumpBonesRequest>()
			.add_plugins(GameCommandPlugin::<ConceptsCommand>::with_config(ui::ui_config()))
			.add_plugins(
				bevy::pbr::MaterialPlugin::<checkerboard_material::CheckerboardMaterial>::default(),
			)
			.add_systems(Startup, (camera::setup_camera, setup_lighting, ground::setup_ground))
			.add_systems(
				Update,
				(
					camera::camera_controller,
					sync_preview.after(capture_command_line_input::<ConceptsCommand>),
					build_rig_bone_map,
					attach_parts_to_sockets.after(build_rig_bone_map),
					remap_part_skin_to_rig.after(attach_parts_to_sockets),
					prune_duplicate_part_scenes.after(remap_part_skin_to_rig),
					init_limb_animators.after(build_rig_bone_map),
					animate_body_rig.after(init_limb_animators),
					dump_bones_to_console,
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			)
			.add_systems(PostUpdate, maintain_resolved_pose);
	}
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
