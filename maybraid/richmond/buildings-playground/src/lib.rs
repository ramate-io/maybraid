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
use commands::RequestMeshStats;
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::ui::GameCommandStatusText;
use ground::setup_ground;
use mesh_ref::MeshRefPlugin;
use preview::{present_preview_lod, track_camera_lod, CameraLodState, CachedPreview};
use richmond_building_components::FurnitureWireframePlugin;

pub struct RichmondBuildingsPlaygroundPlugin;

impl Plugin for RichmondBuildingsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PreviewConfig>()
			.init_resource::<CameraLodState>()
			.init_resource::<CachedPreview>()
			.add_plugins((
				MeshRefPlugin,
				FurnitureWireframePlugin,
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()),
			))
			.add_systems(Startup, (camera::setup_camera, setup_lighting, setup_ground))
			.add_systems(
				Update,
				(
					camera::camera_controller,
					track_camera_lod.after(camera::camera_controller),
					present_preview_lod
						.after(track_camera_lod)
						.after(capture_command_line_input::<PlaygroundCommand>),
					apply_mesh_stats.after(present_preview_lod),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn apply_mesh_stats(
	mut commands: Commands,
	mut status: ResMut<GameCommandStatusText>,
	mesh_assets: Res<Assets<Mesh>>,
	requests: Query<Entity, With<RequestMeshStats>>,
	mesh_entities: Query<&Mesh3d>,
) {
	for entity in &requests {
		let mut mesh_count = 0usize;
		let mut missing = 0usize;
		let mut vertices = 0usize;
		let mut indices = 0usize;
		let mut triangles = 0usize;
		let mut unique_handles = std::collections::HashSet::new();

		for mesh3d in &mesh_entities {
			mesh_count += 1;
			unique_handles.insert(mesh3d.0.id());
			let Some(mesh) = mesh_assets.get(&mesh3d.0) else {
				missing += 1;
				continue;
			};
			let verts = mesh.count_vertices();
			let index_count = mesh.indices().map(|i| i.len()).unwrap_or(verts);
			vertices += verts;
			indices += index_count;
			triangles += index_count / 3;
		}

		status.0 = format!(
			"stats mesh: entities={mesh_count} unique_handles={} missing={missing} verts={vertices} indices={indices} tris={triangles}",
			unique_handles.len()
		);
		info!("{}", status.0);
		commands.entity(entity).despawn();
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
