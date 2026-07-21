//! Interactive Durham terrain models playground.

pub mod camera;
pub mod commands;
mod debug_bounds;
mod player;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use player::PlaygroundMode;

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use camera::{camera_controller, refocus_camera_on_layout, setup_camera};
use commands::{
	PendingCellLayoutPatch, RequestCellShow, RequestModeCharacter, RequestModeFree,
};
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin};
use debug_bounds::{
	draw_chunk_boundary_boxes, setup_cell_location_hud, update_cell_location_hud,
	PlaygroundDebugOverlay,
};
use bevy::math::{IVec2, UVec2};
use durham_terrain_models::{
	AvianTerrainIndex, BaseTerrainNoise, ComposedTerrain, DurhamTerrainModelsPlugin, Terrain,
	TerrainCellLayout, TerrainConfig, TerrainEntryStore, TerrainPresentationAssets,
	TerrainRegionPresenter, TerrainStoreView,
};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText};
use lod::gen::{GeneratingSpatialIndex, RegionPresenter};
use lod::lod_ref::LodRef;
use player::{respawn_player_on_layout, Player, PlayerPlugin};
use render_item::mesh::handle::EnforceCachingPlugin;
use std::f32::consts::PI;

/// Grid radius for the playground request region (`[-r, r]` → `2r + 1` cells).
///
/// About 2× the durham default naturescapes radius (12), so the visible patch
/// is roughly doubled in each horizontal extent.
const PLAYGROUND_GRID_RADIUS_XZ: i32 = 24;

/// Base noise used for camera / player height before (and alongside) generation.
#[derive(Resource)]
pub struct WorldBaseTerrain(pub BaseTerrainNoise);

fn playground_cell_layout() -> TerrainCellLayout {
	let mut layout = TerrainCellLayout::default();
	layout.origin = IVec2::new(-PLAYGROUND_GRID_RADIUS_XZ, -PLAYGROUND_GRID_RADIUS_XZ);
	let n = (2 * PLAYGROUND_GRID_RADIUS_XZ + 1) as u32;
	layout.extents = UVec2::new(n, n);
	layout
}

#[derive(Resource)]
struct TerrainPresentationDirty(bool);

#[derive(Resource, Default)]
struct TerrainPresentPending(bool);

pub struct TerrainModelsPlaygroundPlugin;

impl Plugin for TerrainModelsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		let config = TerrainConfig::new(42);
		let base = BaseTerrainNoise::from_config(&config);

		app.add_plugins(DurhamTerrainModelsPlugin)
			.add_plugins(DurhamTerrainShaderPlugin)
			.add_plugins(EnforceCachingPlugin::<ComposedTerrain, DurhamTerrainShader>::default())
			.add_plugins(
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
					.with_drawer_config(GameCommandDrawerConfig {
						open_at_start: false,
						toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
						..default()
					}),
			)
			.add_plugins(PlayerPlugin)
			.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(config.clone())
			.insert_resource(WorldBaseTerrain(base))
			// After `DurhamTerrainModelsPlugin` so this replaces the default layout.
			.insert_resource(playground_cell_layout())
			.insert_resource(TerrainPresentationDirty(true))
			.init_resource::<TerrainPresentPending>()
			.init_resource::<PlaygroundDebugOverlay>()
			.add_systems(
				Startup,
				(
					setup_camera,
					setup_lighting,
					setup_presentation_assets,
					setup_cell_location_hud,
				),
			)
			.add_systems(
				Update,
				(
					camera_controller,
					apply_cell_commands.after(capture_command_line_input::<PlaygroundCommand>),
					apply_mode_commands.after(apply_cell_commands),
					generate_cells.after(apply_mode_commands),
					present_cells.after(generate_cells),
					draw_chunk_boundary_boxes.after(present_cells),
					update_cell_location_hud.after(draw_chunk_boundary_boxes),
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

pub(crate) fn setup_presentation_assets(
	mut commands: Commands,
	mut materials: ResMut<Assets<DurhamTerrainShader>>,
	config: Res<TerrainConfig>,
) {
	let material = materials.add(DurhamTerrainShader::default());
	commands.insert_resource(TerrainPresentationAssets {
		config: config.clone(),
		material,
		res_2: 5,
	});
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

fn apply_mode_commands(
	mut commands: Commands,
	mut mode: ResMut<PlaygroundMode>,
	mut status: ResMut<GameCommandStatusText>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	free: Query<Entity, With<RequestModeFree>>,
	character: Query<Entity, With<RequestModeCharacter>>,
	mut players: Query<(&mut Transform, &mut LinearVelocity), With<Player>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), (With<Camera3d>, Without<Player>)>,
) {
	for entity in &free {
		*mode = PlaygroundMode::Free;
		status.0 = "mode free".into();
		if let Ok((mut cam_t, mut controller)) = cameras.single_mut() {
			refocus_camera_on_layout(&layout, &base.0, &mut cam_t, &mut controller);
		}
		commands.entity(entity).despawn();
	}

	for entity in &character {
		*mode = PlaygroundMode::Character;
		status.0 = "mode character — WASD move, mouse look, Space jump".into();
		if let Ok((mut transform, mut velocity)) = players.single_mut() {
			respawn_player_on_layout(&layout, &base.0, &mut transform, &mut velocity);
		}
		commands.entity(entity).despawn();
	}
}

fn generate_cells(
	mut index: AvianTerrainIndex,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mode: Res<PlaygroundMode>,
	mut world_base: ResMut<WorldBaseTerrain>,
	mut cameras: Query<(&mut Transform, &mut CameraController), (With<Camera3d>, Without<Player>)>,
	mut players: Query<(&mut Transform, &mut LinearVelocity), With<Player>>,
) {
	if !dirty.0 {
		return;
	}

	index.clear();

	let layout = index.layout().clone();
	let region = layout.request_region();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};

	let _ =
		GeneratingSpatialIndex::<Terrain>::get_or_generate_region(&mut index, region, &lod_ref);

	if let Some(base) = index.base_noise() {
		world_base.0 = base.clone();
	}

	if let Ok((mut transform, mut velocity)) = players.single_mut() {
		respawn_player_on_layout(&layout, &world_base.0, &mut transform, &mut velocity);
	}

	if *mode == PlaygroundMode::Free {
		if let Ok((mut transform, mut controller)) = cameras.single_mut() {
			refocus_camera_on_layout(&layout, &world_base.0, &mut transform, &mut controller);
		}
	}

	dirty.0 = false;
	pending.0 = true;
}

fn present_cells(
	mut presenter: TerrainRegionPresenter,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	mut pending: ResMut<TerrainPresentPending>,
) {
	if !pending.0 {
		return;
	}

	let region = layout.request_region();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	let view = TerrainStoreView::new(&store, &layout);
	RegionPresenter::<Terrain, _>::present(&mut presenter, &view, region, &lod_ref);

	pending.0 = false;
}
