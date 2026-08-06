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

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use commands::RequestMeshStats;
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::ui::GameCommandStatusText;
use ground::setup_ground;
use lod::{add_fine_pass_cull_for, add_fine_pass_for, LodFinePassPlugin, LodFinePassSystems};
use preview::{
	draw_connecting_hall_gizmos, draw_connecting_shells_gizmos, draw_label_text_gizmos,
	draw_opening_plan_gizmos, draw_roof_complex_gizmos, present_preview_lod, CachedPreview,
};
use richmond_building_components::{
	apply_parent_confines, update_panel_host_levels, update_partition_host_levels,
	update_roof_host_levels, FurnitureWireframePlugin, LabelWireframePlugin, WarmAssetLodRoots,
};
use richmond_buildings::wizards_tower::{TowerSilhouettePlugin, WizardsTower};
use scene_ref::SceneRefPlugin;

pub struct RichmondBuildingsPlaygroundPlugin;

impl Plugin for RichmondBuildingsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PreviewConfig>()
			.init_resource::<CachedPreview>()
			.add_plugins((
				SceneRefPlugin,
				FurnitureWireframePlugin,
				LabelWireframePlugin,
				TowerSilhouettePlugin,
				LodFinePassPlugin,
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()),
			));
		add_fine_pass_for::<WizardsTower>(app);
		// Probe updates LodSceneLevel; WarmAssetLodRoots handles spawn + cull.
		add_fine_pass_cull_for::<WarmAssetLodRoots>(app);
		app.add_systems(Startup, (camera::setup_camera, setup_lighting, setup_ground))
			.add_systems(
				Update,
				(
					camera::camera_controller.before(LodFinePassSystems::Track),
					present_preview_lod
						.after(LodFinePassSystems::Track)
						.after(capture_command_line_input::<PlaygroundCommand>),
					draw_connecting_hall_gizmos.after(present_preview_lod),
					draw_connecting_shells_gizmos.after(present_preview_lod),
					draw_opening_plan_gizmos.after(present_preview_lod),
					draw_label_text_gizmos.after(present_preview_lod),
					draw_roof_complex_gizmos.after(present_preview_lod),
					update_partition_host_levels.in_set(LodFinePassSystems::UpdateLevels),
					update_panel_host_levels.in_set(LodFinePassSystems::UpdateLevels),
					update_roof_host_levels.in_set(LodFinePassSystems::UpdateLevels),
					apply_parent_confines.after(LodFinePassSystems::Cull),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			)
			.add_systems(PostUpdate, apply_mesh_stats.after(VisibilitySystems::CheckVisibility));
	}
}

/// Count total vs view-visible mesh triangles (`ViewVisibility`).
fn apply_mesh_stats(
	mut commands: Commands,
	mut status: ResMut<GameCommandStatusText>,
	mesh_assets: Res<Assets<Mesh>>,
	requests: Query<Entity, With<RequestMeshStats>>,
	mesh_entities: Query<(&Mesh3d, &ViewVisibility)>,
) {
	for entity in &requests {
		let mut total_entities = 0usize;
		let mut visible_entities = 0usize;
		let mut missing = 0usize;
		let mut total_tris = 0usize;
		let mut visible_tris = 0usize;
		let mut unique_handles = std::collections::HashSet::new();
		let mut visible_unique_handles = std::collections::HashSet::new();

		for (mesh3d, view_visibility) in &mesh_entities {
			total_entities += 1;
			unique_handles.insert(mesh3d.0.id());
			let Some(mesh) = mesh_assets.get(&mesh3d.0) else {
				missing += 1;
				continue;
			};
			let verts = mesh.count_vertices();
			let index_count = mesh.indices().map(|i| i.len()).unwrap_or(verts);
			let tris = index_count / 3;
			total_tris += tris;
			if view_visibility.get() {
				visible_entities += 1;
				visible_unique_handles.insert(mesh3d.0.id());
				visible_tris += tris;
			}
		}

		status.0 = format!(
			"stats mesh:\n  total_tris={total_tris}\n  visible_tris={visible_tris}\n  entities={total_entities} visible_entities={visible_entities} unique_handles={} visible_unique={} missing={missing}",
			unique_handles.len(),
			visible_unique_handles.len(),
		);
		info!("{}", status.0);
		commands.entity(entity).despawn();
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	// Key light (casts shadows).
	commands.spawn((
		DirectionalLight { illuminance: 10000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
	));
	// Fill from the opposite side (no shadows) to soften contrast.
	commands.spawn((
		DirectionalLight { illuminance: 3500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 5.0, -PI / 4.0, 0.0)),
	));
	// Soft bounce / skylight fill.
	commands.spawn((
		DirectionalLight { illuminance: 1800.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 6.0, PI + PI / 3.0, 0.0)),
	));
}
