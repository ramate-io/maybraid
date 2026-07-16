//! Interactive Durham terrain models playground.

pub mod camera;
pub mod commands;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use camera::{camera_controller, refocus_camera_on_layout, setup_camera};
use commands::{PendingCellLayoutPatch, RequestCellShow};
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin};
use durham_terrain_models::{
	cascade_chunk_for_cell, create_terrain, AvianTerrainIndex, ComposedTerrain,
	DurhamTerrainModelsPlugin, Terrain, TerrainCellLayout, TerrainConfig, TerrainRenderItem,
};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::ui::GameCommandStatusText;
use lod::gen::GeneratingSpatialIndex;
use lod::lod_ref::LodRef;
use render_item::mesh::handle::EnforceCachingPlugin;
use render_item::RenderItem;
use std::f32::consts::PI;

#[derive(Resource)]
pub struct WorldTerrainSdf(pub ComposedTerrain);

#[derive(Resource)]
struct TerrainMaterial(Handle<DurhamTerrainShader>);

#[derive(Resource)]
struct TerrainPresentationDirty(bool);

/// Marks mesh entities spawned for the current cell presentation pass.
#[derive(Component)]
struct PresentedTerrainMesh;

pub struct TerrainModelsPlaygroundPlugin;

impl Plugin for TerrainModelsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		let config = TerrainConfig::new(42);
		let sdf = create_terrain(&config);

		app.add_plugins(DurhamTerrainModelsPlugin)
			.add_plugins(DurhamTerrainShaderPlugin)
			.add_plugins(EnforceCachingPlugin::<ComposedTerrain, DurhamTerrainShader>::default())
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
			.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(config)
			.insert_resource(WorldTerrainSdf(sdf))
			.insert_resource(TerrainPresentationDirty(true))
			.add_systems(Startup, (setup_camera, setup_lighting, setup_material))
			.add_systems(
				Update,
				(
					camera_controller,
					apply_cell_commands.after(capture_command_line_input::<PlaygroundCommand>),
					generate_and_present_cells.after(apply_cell_commands),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	commands.spawn((
		DirectionalLight {
			illuminance: 12_000.0,
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight {
			illuminance: 500.0,
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}

fn setup_material(mut commands: Commands, mut materials: ResMut<Assets<DurhamTerrainShader>>) {
	let handle = materials.add(DurhamTerrainShader::default());
	commands.insert_resource(TerrainMaterial(handle));
}

fn apply_cell_commands(
	mut commands: Commands,
	mut layout: ResMut<TerrainCellLayout>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut status: ResMut<GameCommandStatusText>,
	patches: Query<(Entity, &PendingCellLayoutPatch)>,
	shows: Query<Entity, With<RequestCellShow>>,
) {
	for entity in &shows {
		status.0 = format!(
			"cells size={:.1} origin=({}, {}) extents={}×{}",
			layout.cell_size, layout.origin.x, layout.origin.y, layout.extents.x, layout.extents.y
		);
		commands.entity(entity).despawn();
	}

	for (entity, patch) in &patches {
		if let Some(size) = patch.size {
			if size > 0.0 {
				layout.cell_size = size;
			}
		}
		if let Some(x) = patch.origin_x {
			layout.origin.x = x;
		}
		if let Some(z) = patch.origin_z {
			layout.origin.y = z;
		}
		if let Some(x) = patch.extent_x {
			layout.extents.x = x.max(1);
		}
		if let Some(z) = patch.extent_z {
			layout.extents.y = z.max(1);
		}
		dirty.0 = true;
		status.0 = format!(
			"cells size={:.1} origin=({}, {}) extents={}×{} (regen)",
			layout.cell_size, layout.origin.x, layout.origin.y, layout.extents.x, layout.extents.y
		);
		commands.entity(entity).despawn();
	}
}

fn generate_and_present_cells(
	mut index: AvianTerrainIndex,
	mut commands: Commands,
	mut dirty: ResMut<TerrainPresentationDirty>,
	world_sdf: Res<WorldTerrainSdf>,
	material: Res<TerrainMaterial>,
	presented: Query<Entity, With<PresentedTerrainMesh>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	if !dirty.0 {
		return;
	}

	for entity in &presented {
		commands.entity(entity).despawn();
	}
	index.clear();

	// Clone layout up front: `AvianTerrainIndex` already owns `ResMut<TerrainCellLayout>`.
	let layout = index.layout().clone();
	let region = layout.request_region();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};

	let cells =
		GeneratingSpatialIndex::<Terrain>::get_or_generate_region(&mut index, region, &lod_ref);
	let cell_count = cells.len();

	let render_item =
		TerrainRenderItem::new(world_sdf.0.clone(), MeshMaterial3d(material.0.clone()));

	for (_id, bounds) in cells {
		let chunk = cascade_chunk_for_cell(bounds, 5);
		for entity in render_item.spawn_render_items(&mut commands, &chunk, Transform::IDENTITY) {
			commands.entity(entity).insert(PresentedTerrainMesh);
		}
	}

	if let Ok((mut transform, mut controller)) = cameras.single_mut() {
		refocus_camera_on_layout(&layout, &world_sdf.0, &mut transform, &mut controller);
	}

	dirty.0 = false;
	info!(
		"Presented {cell_count} terrain cells (size={:.1}, origin=({}, {}), extents={}×{})",
		layout.cell_size, layout.origin.x, layout.origin.y, layout.extents.x, layout.extents.y
	);
}
