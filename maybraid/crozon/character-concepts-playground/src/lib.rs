//! Command-driven playground for the Character Concepts Screen.
//!
//! This crate is the first executable surface for the concept plan. Commands are
//! temporary stand-ins for future UI fields; they spawn through
//! `crozon-characters` LodScene recipes (`Config::clothed()`).

mod animation;
mod camera_focus;
mod character_lod;
pub mod commands;
mod diagnostics;
mod focus;
mod focus_reference;
mod material;
mod menu_listeners;
mod preview;
mod preview_color;
mod scale_reference;
mod skinning;
mod species_session;
mod thumbnail;
mod ui;

pub use commands::{ConceptsCommand, CONCEPTS_CLI_NAME};
pub use diagnostics::fps_debug_enabled;
pub use game_commands::command::PendingStartupCommand;

use bevy::app::SceneSpawnerSystems;
use bevy::prelude::*;
use bevy_character_ui_menu_renderer::CharacterMenuRendererPlugin;
use camera_controls::look::{CameraLookConfig, CameraLookPlugin};
use crozon_character_playground::camera;
use crozon_character_ui_menus::CharacterMenu;
use game_commands::command::{capture_command_line_input, GameCommandPlugin};

use animation::{animate_body_rig, init_limb_animators};
use camera_focus::{apply_camera_suggestion, PendingCameraFocus};
use character_lod::CharacterLodPlugin;
use crozon_characters::{
	build_rig_bone_map, fulfill_skin_ref_roots, fulfill_socket_ref_roots,
	prune_duplicate_part_scenes, remap_part_skin_to_rig, CharacterHostSystems,
};
use focus::animate_focused_preview_asset;
use focus_reference::{sync_focus_reference, FocusReferenceSyncState};
use lod::{LodRefreshSystems, LodViewer};
use material::apply_preview_colors;
use material::PreviewColorMaterials;
use menu_listeners::{
	dispatch_menu_interactions, init_character_menu_state, on_character_menu_event,
	sync_menu_state_from_config, CharacterMenuState,
};
use preview::{
	preview_pass_ready, reveal_ready_preview, stamp_lod_character_preview, sync_preview,
	tick_preview_respawn_cooldown, ConceptPreviewConfig, ConceptPreviewSyncState,
	PreviewRespawnCooldown, PreviewRevealDebugState,
};
use skinning::{
	attach_focus_reference_to_sockets, attach_parts_to_sockets, dump_bones_to_console,
	maintain_resolved_pose, DumpBonesRequest,
};
use species_session::{
	ensure_species_camera_focus, persist_species_session, CameraFocusBootState, SpeciesSessionState,
};

pub struct CrozonCharacterConceptsPlaygroundPlugin;

impl Plugin for CrozonCharacterConceptsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		if diagnostics::fps_debug_enabled() {
			app.add_plugins(diagnostics::FpsDiagnosticsPlugin);
		}
		app.init_resource::<ConceptPreviewConfig>()
			.init_resource::<ConceptPreviewSyncState>()
			.init_resource::<FocusReferenceSyncState>()
			.init_resource::<PreviewRespawnCooldown>()
			.init_resource::<PreviewRevealDebugState>()
			.init_resource::<DumpBonesRequest>()
			.init_resource::<PendingCameraFocus>()
			.init_resource::<SpeciesSessionState>()
			.init_resource::<CameraFocusBootState>()
			.init_resource::<PreviewColorMaterials>()
			.init_resource::<thumbnail::ThumbnailCache>()
			.init_resource::<ui::CreatorUiState>()
			.init_resource::<ui::CreatorUiSyncState>()
			.init_resource::<CharacterMenuState>()
			.add_plugins(CharacterMenuRendererPlugin::<CharacterMenu>::default())
			.add_plugins(CameraLookPlugin::new(CameraLookConfig {
				enabled_at_start: false,
				..CameraLookConfig::default()
			}))
			.add_plugins(CharacterLodPlugin)
			.add_plugins(
				GameCommandPlugin::<ConceptsCommand>::with_config(ui::ui_config())
					.with_drawer_config(ui::drawer_config()),
			)
			.add_observer(ui::on_creator_ui_scroll)
			.add_systems(
				Startup,
				(
					camera::setup_camera,
					add_lod_viewer_to_camera.after(camera::setup_camera),
					setup_lighting,
					scale_reference::setup_scale_reference,
					init_character_menu_state,
					ui::setup_creator_ui,
				),
			)
			.add_systems(
				Update,
				(
					tick_preview_respawn_cooldown,
					persist_species_session,
					ensure_species_camera_focus.after(persist_species_session),
					sync_menu_state_from_config,
					dispatch_menu_interactions,
					on_character_menu_event.after(dispatch_menu_interactions),
					ui::sync_creator_ui.after(dispatch_menu_interactions),
					ui::refresh_creator_ui_display.after(ui::sync_creator_ui),
					camera::camera_controller,
					ui::send_creator_ui_scroll_events,
					sync_preview
						.after(capture_command_line_input::<ConceptsCommand>)
						.after(on_character_menu_event),
					sync_focus_reference.after(sync_preview),
					stamp_lod_character_preview
						.after(sync_preview)
						.after(LodRefreshSystems::Fulfill),
					animate_focused_preview_asset
						.after(dispatch_menu_interactions)
						.before(sync_preview),
					build_rig_bone_map
						.after(sync_focus_reference)
						.after(stamp_lod_character_preview),
					maintain_resolved_pose.after(build_rig_bone_map),
				),
			)
			.add_systems(
				Update,
				(
					attach_focus_reference_to_sockets.after(build_rig_bone_map),
					attach_parts_to_sockets.after(build_rig_bone_map).run_if(preview_pass_ready),
					fulfill_socket_ref_roots
						.after(build_rig_bone_map)
						.after(CharacterHostSystems::InvalidateRefs)
						.run_if(preview_pass_ready),
					fulfill_skin_ref_roots
						.after(fulfill_socket_ref_roots)
						.run_if(preview_pass_ready),
					remap_part_skin_to_rig
						.after(attach_parts_to_sockets)
						.after(fulfill_skin_ref_roots)
						.after(SceneSpawnerSystems::WorldInstanceSpawn)
						.run_if(preview_pass_ready),
					prune_duplicate_part_scenes
						.after(remap_part_skin_to_rig)
						.run_if(preview_pass_ready),
					reveal_ready_preview
						.after(maintain_resolved_pose)
						.after(prune_duplicate_part_scenes)
						.run_if(preview_pass_ready),
					apply_preview_colors
						.after(prune_duplicate_part_scenes)
						.run_if(preview_pass_ready),
					init_limb_animators.after(maintain_resolved_pose).run_if(preview_pass_ready),
					animate_body_rig.after(init_limb_animators).run_if(preview_pass_ready),
					dump_bones_to_console,
					thumbnail::sync_thumbnail_camera_activity.after(ui::sync_creator_ui),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			)
			.add_systems(
				PostUpdate,
				(
					maintain_resolved_pose.before(TransformSystems::Propagate),
					// Runs after propagation so shadow-rig socket globals reflect
					// the pose written this frame.
					apply_camera_suggestion
						.after(TransformSystems::Propagate)
						.after(maintain_resolved_pose),
				),
			);
	}
}

fn add_lod_viewer_to_camera(
	mut commands: Commands,
	cameras: Query<Entity, (With<Camera3d>, Without<LodViewer>)>,
) {
	for entity in &cameras {
		commands.entity(entity).insert(LodViewer);
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
