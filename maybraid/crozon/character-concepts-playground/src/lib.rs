//! Command-driven playground for the Character Concepts Screen.
//!
//! This crate is the first executable surface for the concept plan. Commands are
//! temporary stand-ins for future UI fields; they still resolve through
//! `crozon-characters` before any Bevy entities are spawned.

mod animation;
mod camera_focus;
pub mod commands;
mod focus;
mod ground;
mod material;
mod preview;
mod skinning;
mod thumbnail;
mod ui;

pub use commands::{ConceptsCommand, CONCEPTS_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use camera_controls::look::{CameraLookConfig, CameraLookPlugin};
use crozon_character_playground::{camera, checkerboard_material};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};

use animation::{animate_body_rig, init_limb_animators};
use camera_focus::{apply_camera_suggestion, PendingCameraFocus};
use focus::animate_focused_preview_asset;
use material::apply_preview_colors;
use material::PreviewColorMaterials;
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
			.init_resource::<PendingCameraFocus>()
			.init_resource::<PreviewColorMaterials>()
			.init_resource::<thumbnail::ThumbnailCache>()
			.init_resource::<ui::CreatorUiState>()
			.init_resource::<ui::CreatorUiSyncState>()
			.add_plugins(CameraLookPlugin::new(CameraLookConfig {
				enabled_at_start: false,
				..CameraLookConfig::default()
			}))
			.add_plugins(
				GameCommandPlugin::<ConceptsCommand>::with_config(ui::ui_config())
					.with_drawer_config(ui::drawer_config()),
			)
			.add_plugins(
				bevy::pbr::MaterialPlugin::<checkerboard_material::CheckerboardMaterial>::default(),
			)
			.add_observer(ui::on_creator_ui_scroll)
			.add_systems(
				Startup,
				(camera::setup_camera, setup_lighting, ground::setup_ground, ui::setup_creator_ui),
			)
			.add_systems(
				Update,
				(
					ui::sync_creator_ui,
					camera::camera_controller,
					ui::react_creator_ui.after(ui::sync_creator_ui),
					ui::send_creator_ui_scroll_events,
					sync_preview
						.after(capture_command_line_input::<ConceptsCommand>)
						.after(ui::react_creator_ui),
					animate_focused_preview_asset
						.after(ui::react_creator_ui)
						.before(sync_preview),
					build_rig_bone_map.after(sync_preview),
					attach_parts_to_sockets.after(build_rig_bone_map),
					remap_part_skin_to_rig.after(attach_parts_to_sockets),
					prune_duplicate_part_scenes.after(remap_part_skin_to_rig),
					apply_preview_colors.after(prune_duplicate_part_scenes),
					init_limb_animators.after(build_rig_bone_map),
					animate_body_rig.after(init_limb_animators),
					apply_camera_suggestion
						.after(ui::react_creator_ui)
						.after(build_rig_bone_map),
					dump_bones_to_console,
					thumbnail::sync_thumbnail_camera_activity.after(ui::sync_creator_ui),
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
