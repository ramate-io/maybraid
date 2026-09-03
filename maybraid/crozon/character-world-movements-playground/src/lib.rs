//! Small Durham patch for iterating Crozon locomotion on real ground.

pub mod camera;
pub mod character;
pub mod commands;
mod pitch;
mod player;
mod ui;

pub use camera::CameraController;
pub use character::CharacterSpecies;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use player::PlaygroundMode;

use avian3d::prelude::LinearVelocity;
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use camera::{camera_controller, refocus_camera_on_layout, setup_camera};
use character::{
	apply_set_character, apply_stampede, drive_player_locomotion, respawn_stampede_members,
	StampedeMember,
};
use commands::{RequestModeCharacter, RequestModeFree};
use crozon_characters::{CharacterHostsPlugin, CharacterMotionSystems, LocomotionCapsule};
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin, RefractionWater};
use durham_terrain_models::{
	AvianTerrainIndex, BaseTerrainNoise, ComposedWater, DurhamTerrainModelsPlugin, Terrain,
	TerrainCellLayout, TerrainConfig, TerrainEntryStore, TerrainMeshBuilder, TerrainMeshLodBand,
	TerrainPresentationAssets, TerrainRegionPresenter, TerrainStoreView, Water,
	WaterPresentationAssets, WaterRegionPresenter, WaterStoreView,
};
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText};
use lod::gen::{GeneratingSpatialIndex, RegionPresenter, SpatialIndex};
use lod::lod_ref::LodRef;
use pitch::{apply_avian_terrain_pitch, sync_suspend_terrain_pitch};
use player::{respawn_player_on_layout, Player, PlayerControlSystems, PlayerPlugin};
use render_item::mesh::handle::EnforceCachingPlugin;
use std::f32::consts::PI;

/// Fine-grid half-extent in base cells. 4×4 cells → ~640 m at the Durham cell size.
const PLAYGROUND_FINE_HALF_EXTENT_CELLS: i32 = 2;

fn playground_lod_bands() -> Vec<TerrainMeshLodBand> {
	vec![TerrainMeshLodBand { max_radius_cells: PLAYGROUND_FINE_HALF_EXTENT_CELLS, res_2: 5 }]
}

/// Base noise used for camera / player height before (and alongside) generation.
#[derive(Resource)]
pub struct WorldBaseTerrain(pub BaseTerrainNoise);

fn playground_cell_layout() -> TerrainCellLayout {
	let mut layout = TerrainCellLayout::default();
	layout.origin =
		IVec2::new(-PLAYGROUND_FINE_HALF_EXTENT_CELLS, -PLAYGROUND_FINE_HALF_EXTENT_CELLS);
	let n = (2 * PLAYGROUND_FINE_HALF_EXTENT_CELLS) as u32;
	layout.extents = UVec2::new(n, n);
	layout.outer_rings.clear();
	layout
}

#[derive(Resource)]
struct TerrainPresentationDirty(bool);

#[derive(Resource, Default)]
struct TerrainPresentPending(bool);

pub struct CharacterWorldMovementsPlaygroundPlugin;

impl Plugin for CharacterWorldMovementsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		let config = TerrainConfig::new(42);
		let base = BaseTerrainNoise::from_config(&config);

		app.add_plugins(DurhamTerrainModelsPlugin)
			.add_plugins(DurhamTerrainShaderPlugin)
			.add_plugins(EnforceCachingPlugin::<TerrainMeshBuilder, DurhamTerrainShader>::default())
			.add_plugins(EnforceCachingPlugin::<ComposedWater, RefractionWater>::default())
			.add_plugins(
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
					.with_drawer_config(GameCommandDrawerConfig {
						open_at_start: false,
						toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
						..default()
					}),
			)
			.add_plugins(PlayerPlugin)
			.add_plugins(CharacterHostsPlugin)
			.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(config.clone())
			.insert_resource(WorldBaseTerrain(base))
			.insert_resource(playground_cell_layout())
			.insert_resource(TerrainPresentationDirty(true))
			.init_resource::<TerrainPresentPending>()
			.add_systems(Startup, (setup_camera, setup_lighting, setup_presentation_assets))
			.add_systems(
				Update,
				(
					camera_controller,
					apply_set_character.after(capture_command_line_input::<PlaygroundCommand>),
					apply_stampede.after(apply_set_character),
					apply_mode_commands.after(apply_stampede),
					generate_cells.after(apply_mode_commands),
					present_cells.after(generate_cells),
					drive_player_locomotion
						.after(PlayerControlSystems)
						.before(CharacterMotionSystems::Anim),
					sync_suspend_terrain_pitch.after(PlayerControlSystems),
					apply_avian_terrain_pitch
						.in_set(CharacterMotionSystems::Elevation)
						.after(drive_player_locomotion)
						.after(sync_suspend_terrain_pitch),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	commands.spawn((
		DirectionalLight { illuminance: 12_000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 2_500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}

fn setup_presentation_assets(
	mut commands: Commands,
	mut terrain_materials: ResMut<Assets<DurhamTerrainShader>>,
	mut water_materials: ResMut<Assets<RefractionWater>>,
	config: Res<TerrainConfig>,
) {
	let material = terrain_materials.add(DurhamTerrainShader::default());
	commands.insert_resource(TerrainPresentationAssets {
		config: config.clone(),
		material,
		lod_bands: playground_lod_bands(),
		outer_add_walls: true,
		fine_grid_max_radius: Some(PLAYGROUND_FINE_HALF_EXTENT_CELLS),
		macro_seam_half_extents: Vec::new(),
		macro_cell_min_size: None,
		macro_res_2: None,
	});
	commands.insert_resource(WaterPresentationAssets {
		material: water_materials.add(RefractionWater::default()),
	});
}

fn apply_mode_commands(
	mut commands: Commands,
	mut mode: ResMut<PlaygroundMode>,
	mut status: ResMut<GameCommandStatusText>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	store: Res<TerrainEntryStore>,
	free: Query<Entity, With<RequestModeFree>>,
	character: Query<Entity, With<RequestModeCharacter>>,
	mut players: Query<(&mut Transform, &mut LinearVelocity), With<Player>>,
	mut herd: Query<
		(&StampedeMember, &LocomotionCapsule, &mut Transform, &mut LinearVelocity),
		(Without<Player>, Without<Camera3d>),
	>,
	mut cameras: Query<
		(&mut Transform, &mut CameraController),
		(With<Camera3d>, Without<Player>, Without<StampedeMember>),
	>,
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
			let center = layout.region_center_xz();
			let elevation = store
				.composed_height_at(&layout, center.x, center.z)
				.unwrap_or_else(|| base.0.height_at(center.x, center.z));
			respawn_player_on_layout(&layout, elevation, &mut transform, &mut velocity);
		}
		respawn_stampede_members(
			&layout,
			|x, z| {
				store
					.composed_height_at(&layout, x, z)
					.unwrap_or_else(|| base.0.height_at(x, z))
			},
			&mut herd,
		);
		commands.entity(entity).despawn();
	}
}

fn generate_cells(
	mut index: AvianTerrainIndex,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mode: Res<PlaygroundMode>,
	mut world_base: ResMut<WorldBaseTerrain>,
	mut cameras: Query<
		(&mut Transform, &mut CameraController),
		(With<Camera3d>, Without<Player>, Without<StampedeMember>),
	>,
	mut players: Query<(&mut Transform, &mut LinearVelocity), With<Player>>,
	mut herd: Query<
		(&StampedeMember, &LocomotionCapsule, &mut Transform, &mut LinearVelocity),
		(Without<Player>, Without<Camera3d>),
	>,
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

	let terrains =
		GeneratingSpatialIndex::<Terrain>::get_or_generate_region(&mut index, region, &lod_ref);
	let waters =
		GeneratingSpatialIndex::<Water>::get_or_generate_region(&mut index, region, &lod_ref);
	let water_fills: usize = waters
		.iter()
		.filter_map(|(id, _)| SpatialIndex::<Water>::get(&index, *id).map(|w| w.fills.len()))
		.sum();
	info!(
		"generated terrain_cells={} water_cells={} water_fills={}",
		terrains.len(),
		waters.len(),
		water_fills
	);

	if let Some(base) = index.base_noise() {
		world_base.0 = base.clone();
	}

	if let Ok((mut transform, mut velocity)) = players.single_mut() {
		let center = layout.region_center_xz();
		let elevation = index
			.composed_height_at(center.x, center.z)
			.unwrap_or_else(|| world_base.0.height_at(center.x, center.z));
		respawn_player_on_layout(&layout, elevation, &mut transform, &mut velocity);
	}
	respawn_stampede_members(
		&layout,
		|x, z| index.composed_height_at(x, z).unwrap_or_else(|| world_base.0.height_at(x, z)),
		&mut herd,
	);

	if *mode == PlaygroundMode::Free {
		if let Ok((mut transform, mut controller)) = cameras.single_mut() {
			refocus_camera_on_layout(&layout, &world_base.0, &mut transform, &mut controller);
		}
	}

	dirty.0 = false;
	pending.0 = true;
}

fn present_cells(
	mut terrain_presenter: TerrainRegionPresenter,
	mut water_presenter: WaterRegionPresenter,
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
	let terrain_view = TerrainStoreView::new(&store, &layout);
	RegionPresenter::<Terrain, _>::present(&mut terrain_presenter, &terrain_view, region, &lod_ref);
	let terrain_wanted = SpatialIndex::<Terrain>::tracked_ids_for(&terrain_view, region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	terrain_presenter.remove_stale(&terrain_wanted);
	let water_view = WaterStoreView::new(&store, &layout);
	RegionPresenter::<Water, _>::present(&mut water_presenter, &water_view, region, &lod_ref);
	let water_wanted = SpatialIndex::<Water>::tracked_ids_for(&water_view, region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	water_presenter.remove_stale(&water_wanted);

	pending.0 = false;
}
