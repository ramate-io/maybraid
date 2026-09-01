//! Small Durham fine-grid patch for iterating Chico groves on real ground.
//!
//! `/forest` streams the unified Chico forest on Durham height (A/B against
//! tiled `/grove`).

mod bump_out;
pub mod camera;
pub mod character;
pub mod commands;
pub mod diagnostics;
mod forest;
mod groves;
mod material_lib;
mod pitch;
pub mod player;
mod ui;

pub use bump_out::DurhamCanopyBumpOutPresenter;
pub use camera::CameraController;
pub use character::{CharacterSpecies, PlayerVisual, RequestSetCharacter};
pub use chico_sbs_trees_playground::forest_stream::ForestStreamSpec;
pub use commands::{GroveKind, PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use diagnostics::{PlaygroundDiag, PlaygroundTimingPlugin, RequestFpsToggle};
pub use forest::DurhamForestPresenter;
pub use game_commands::command::PendingStartupCommand;
pub use material_lib::{VegetationOnTerrainMaterialLib, VegetationOnTerrainMaterialRefPlugin};
pub use player::{
	CharacterCameraFollowEnabled, CharacterLocomotion, MoveWish, MovementAction,
	PadMovementEnabled, Player, PlayerCapsule, PlayerControlSystems, PlayerPlugin, PlaygroundMode,
};

use avian3d::prelude::LinearVelocity;
use bevy::camera::visibility::VisibilitySystems;
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use bump_out::{register_bump_out_lod, stream_canopy_bump_outs};
use camera::{
	camera_controller, refocus_camera_on_elevation, release_modifiers_on_focus_change,
	setup_camera, surface_or_hold,
};
use character::{apply_set_character, drive_player_locomotion};
use chico_bumpout::ChicoBumpOutPlugin;
use chico_groves::DEFAULT_GROVE_EXTENT_XZ;
use chico_sbs_trees_playground::forest_stream::{register_forest_lod, stream_radii_m};
use chico_sbs_trees_playground::register_vegetation_view;
use chico_vegetation_components::{FoliageLodProbe, StickLodProbe};
use commands::{
	RequestForest, RequestGrove, RequestGroveExtent, RequestMeshStats, RequestModeCharacter,
	RequestModeFree, RequestRebuild, RequestTerrainRadius, RequestTileRadius,
};
use crozon_characters::{CharacterHostsPlugin, CharacterMotionSystems};
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin, RefractionWater};
use durham_terrain_models::{
	AvianTerrainIndex, BaseTerrainNoise, ComposedWater, DurhamTerrainModelsPlugin, OuterCellRing,
	Terrain, TerrainCellLayout, TerrainConfig, TerrainEntryStore, TerrainMeshBuilder,
	TerrainMeshLodBand, TerrainPresentationAssets, TerrainRegionPresenter, TerrainStoreView, Water,
	WaterPresentationAssets, WaterRegionPresenter, WaterStoreView, TERRAIN_CELL_SIZE,
};
use forest::stream_durham_forest;
use game_commands::command::{
	capture_command_line_input, GameCommandPlugin, TextEntryBlocked, TextEntryFocus,
};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText};
use groves::{spawn_tiled_groves, GroveRoot};
use lod::gen::{GeneratingSpatialIndex, RegionPresenter};
use lod::lod_ref::LodRef;
use lod::{LodGenerateSystems, LodPresentSystems, LodSceneHost};
use maybraid_input::{PadGameplayEnabled, VirtualPadPlugin, VirtualPadSystems};
use pitch::{apply_avian_terrain_pitch, sync_suspend_terrain_pitch};
use player::{
	holding_elevation, respawn_player_on_layout, snap_player_to_composed_surface,
	AwaitingTerrainSurface,
};
use render_item::mesh::handle::{EnforceCachingPlugin, EnforcedCaches};
use std::f32::consts::PI;
use terrain_chunk_ref::{TerrainChunkRefCache, TerrainChunkRefPlugin};

const DEFAULT_TERRAIN_RADIUS: i32 = 2;
const DEFAULT_TILE_RADIUS: i32 = 1;

/// Fine-grid Chebyshev half-extent (16 × 160 m ≈ 2.6 km). Playable disk from
/// [#675](https://github.com/ramate-io/maybraid/pull/675); bump-outs attach to these
/// cells instead of expanding generate.
const WORLD_FINE_HALF_EXTENT_CELLS: i32 = 16;
/// 2× macro ring past the fine grid (was 4; that disk was ~5 km half-extent).
const WORLD_OUTER_2X_ROWS: i32 = 2;
/// 4× macro ring past the 2× ring.
const WORLD_OUTER_4X_ROWS: i32 = 1;

/// Fine-only patch vs playable world extents (fine grid + macro rings).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerrainCoverage {
	#[default]
	FinePatch,
	PlayableWorld,
}

fn playground_lod_bands(half_extent: i32) -> Vec<TerrainMeshLodBand> {
	vec![TerrainMeshLodBand { max_radius_cells: half_extent.max(1), res_2: 5 }]
}

fn world_lod_bands() -> Vec<TerrainMeshLodBand> {
	vec![
		TerrainMeshLodBand { max_radius_cells: 2, res_2: 5 },
		TerrainMeshLodBand { max_radius_cells: 5, res_2: 3 },
		TerrainMeshLodBand { max_radius_cells: 16, res_2: 2 },
	]
}

fn cell_layout(half_extent: i32) -> TerrainCellLayout {
	let r = half_extent.max(1);
	let mut layout = TerrainCellLayout::default();
	layout.origin = IVec2::new(-r, -r);
	let n = (2 * r) as u32;
	layout.extents = UVec2::new(n, n);
	layout.outer_rings.clear();
	layout
}

fn world_cell_layout() -> TerrainCellLayout {
	let mut layout = TerrainCellLayout::default();
	layout.origin = IVec2::new(-WORLD_FINE_HALF_EXTENT_CELLS, -WORLD_FINE_HALF_EXTENT_CELLS);
	let n = (2 * WORLD_FINE_HALF_EXTENT_CELLS) as u32;
	layout.extents = UVec2::new(n, n);
	layout.outer_rings = vec![
		OuterCellRing { cell_size: 2.0 * TERRAIN_CELL_SIZE, rows: WORLD_OUTER_2X_ROWS },
		OuterCellRing { cell_size: 4.0 * TERRAIN_CELL_SIZE, rows: WORLD_OUTER_4X_ROWS },
	];
	layout
}

fn layout_for(playground: &PlaygroundConfig) -> TerrainCellLayout {
	match playground.coverage {
		TerrainCoverage::FinePatch => cell_layout(playground.terrain_radius),
		TerrainCoverage::PlayableWorld => world_cell_layout(),
	}
}

fn lod_bands_for(playground: &PlaygroundConfig) -> Vec<TerrainMeshLodBand> {
	match playground.coverage {
		TerrainCoverage::FinePatch => playground_lod_bands(playground.terrain_radius),
		TerrainCoverage::PlayableWorld => world_lod_bands(),
	}
}

fn terrain_cells_for_generate_m(generate_m: f32) -> i32 {
	(generate_m / TERRAIN_CELL_SIZE).ceil() as i32
}

/// Base noise used for camera height before (and alongside) generation.
#[derive(Resource)]
pub struct WorldBaseTerrain(pub BaseTerrainNoise);

#[derive(Resource, Clone)]
pub struct PlaygroundConfig {
	pub grove: commands::GroveKind,
	pub terrain_radius: i32,
	pub grove_extent_xz: f32,
	pub tile_radius: i32,
	/// `Some` streams the forest and skips tiled groves.
	pub forest: Option<ForestStreamSpec>,
	pub coverage: TerrainCoverage,
}

impl Default for PlaygroundConfig {
	fn default() -> Self {
		Self {
			grove: commands::GroveKind::MonsterGrass,
			terrain_radius: DEFAULT_TERRAIN_RADIUS,
			grove_extent_xz: DEFAULT_GROVE_EXTENT_XZ,
			tile_radius: DEFAULT_TILE_RADIUS,
			forest: None,
			coverage: TerrainCoverage::FinePatch,
		}
	}
}

impl PlaygroundConfig {
	/// Terrain + forest at playable present / generate extents.
	pub fn world_defaults() -> Self {
		Self {
			grove: commands::GroveKind::MonsterGrass,
			terrain_radius: WORLD_FINE_HALF_EXTENT_CELLS,
			grove_extent_xz: DEFAULT_GROVE_EXTENT_XZ,
			tile_radius: DEFAULT_TILE_RADIUS,
			forest: Some(ForestStreamSpec { stream_radius: 1, ..ForestStreamSpec::default() }),
			coverage: TerrainCoverage::PlayableWorld,
		}
	}
}

#[derive(Resource)]
struct TerrainPresentationDirty(bool);

#[derive(Resource, Default)]
struct TerrainPresentPending(bool);

#[derive(Resource)]
struct GrovesDirty(bool);

pub struct VegetationOnTerrainPlugin {
	pub config: PlaygroundConfig,
	/// When false, the caller owns the command drawer / CLI.
	pub commands: bool,
}

impl Default for VegetationOnTerrainPlugin {
	fn default() -> Self {
		Self { config: PlaygroundConfig::default(), commands: true }
	}
}

impl Plugin for VegetationOnTerrainPlugin {
	fn build(&self, app: &mut App) {
		let config = TerrainConfig::new(42);
		let base = BaseTerrainNoise::from_config(&config);
		let playground = self.config.clone();

		app.add_plugins(DurhamTerrainModelsPlugin)
			.add_plugins(DurhamTerrainShaderPlugin)
			.add_plugins(ChicoBumpOutPlugin)
			.add_plugins(EnforceCachingPlugin::<TerrainMeshBuilder, DurhamTerrainShader>::default())
			.add_plugins(EnforceCachingPlugin::<ComposedWater, RefractionWater>::default());
		let (terrain_handles, terrain_disk) = {
			let caches = app.world().resource::<EnforcedCaches<TerrainMeshBuilder>>();
			(caches.handle_map(), caches.disk_cache())
		};
		app.insert_resource(
			TerrainChunkRefCache::<TerrainMeshBuilder>::new()
				.with_handles(terrain_handles)
				.with_optional_disk(terrain_disk)
				.without_build_on_miss(),
		)
		.add_plugins(TerrainChunkRefPlugin::<TerrainMeshBuilder>::default());
		if self.commands {
			app.add_plugins(
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
					.with_drawer_config(GameCommandDrawerConfig {
						open_at_start: false,
						toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
						..default()
					}),
			);
		}
		register_vegetation_view(app);
		if !app.is_plugin_added::<VegetationOnTerrainMaterialRefPlugin>() {
			app.add_plugins(VegetationOnTerrainMaterialRefPlugin);
		}
		register_forest_lod::<DurhamForestPresenter>(app);
		register_bump_out_lod::<DurhamCanopyBumpOutPresenter>(app);
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		if !app.is_plugin_added::<PlayerPlugin>() {
			app.add_plugins(PlayerPlugin);
		}
		if !app.is_plugin_added::<CharacterHostsPlugin>() {
			app.add_plugins(CharacterHostsPlugin);
		}
		app.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
			.insert_resource(config.clone())
			.insert_resource(WorldBaseTerrain(base))
			.insert_resource(playground.clone())
			.insert_resource(layout_for(&playground))
			.insert_resource(TerrainPresentationDirty(true))
			.init_resource::<TerrainPresentPending>()
			.insert_resource(GrovesDirty(true))
			.add_systems(Startup, (setup_camera, setup_lighting, setup_presentation_assets))
			.add_systems(PreUpdate, sync_pad_gameplay.before(VirtualPadSystems::Produce))
			.add_systems(PostUpdate, apply_mesh_stats.after(VisibilitySystems::CheckVisibility));
		if self.commands {
			app.add_systems(
				Update,
				(
					release_modifiers_on_focus_change.before(camera_controller),
					camera_controller,
					apply_commands.after(capture_command_line_input::<PlaygroundCommand>),
					generate_cells.after(apply_commands),
					present_cells.after(generate_cells),
					spawn_groves.after(present_cells),
					stream_durham_forest
						.after(apply_commands)
						.before(LodGenerateSystems::Produce)
						.before(LodPresentSystems::Produce),
					stream_canopy_bump_outs
						.after(stream_durham_forest)
						.before(LodGenerateSystems::Produce)
						.before(LodPresentSystems::Produce),
					apply_set_character.after(apply_commands),
					apply_mode_commands.after(apply_set_character),
					snap_player_to_composed_surface
						.after(present_cells)
						.after(apply_mode_commands)
						.before(PlayerControlSystems),
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
		} else {
			app.add_systems(
				Update,
				(
					release_modifiers_on_focus_change.before(camera_controller),
					camera_controller,
					generate_cells,
					present_cells.after(generate_cells),
					spawn_groves.after(present_cells),
					stream_durham_forest
						.before(LodGenerateSystems::Produce)
						.before(LodPresentSystems::Produce),
					stream_canopy_bump_outs
						.after(stream_durham_forest)
						.before(LodGenerateSystems::Produce)
						.before(LodPresentSystems::Produce),
					apply_set_character,
					apply_mode_commands.after(apply_set_character),
					snap_player_to_composed_surface
						.after(present_cells)
						.after(apply_mode_commands)
						.before(PlayerControlSystems),
					drive_player_locomotion
						.after(PlayerControlSystems)
						.before(CharacterMotionSystems::Anim),
					sync_suspend_terrain_pitch.after(PlayerControlSystems),
					apply_avian_terrain_pitch
						.in_set(CharacterMotionSystems::Elevation)
						.after(drive_player_locomotion)
						.after(sync_suspend_terrain_pitch),
				),
			);
		}
	}
}

/// Count total vs view-visible mesh triangles (`ViewVisibility`) and LOD probe hosts.
fn apply_mesh_stats(
	mut commands: Commands,
	mut status: Option<ResMut<GameCommandStatusText>>,
	mesh_assets: Res<Assets<Mesh>>,
	requests: Query<Entity, With<RequestMeshStats>>,
	mesh_entities: Query<(&Mesh3d, &ViewVisibility)>,
	foliage_probes: Query<(), With<FoliageLodProbe>>,
	stick_probes: Query<(), With<StickLodProbe>>,
	lod_hosts: Query<(), With<LodSceneHost>>,
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

		let foliage_probes = foliage_probes.iter().count();
		let stick_probes = stick_probes.iter().count();
		let lod_hosts = lod_hosts.iter().count();
		let probes_total = foliage_probes + stick_probes;

		let text = format!(
			"stats mesh:\n  total_tris={total_tris}\n  visible_tris={visible_tris}\n  entities={total_entities} visible_entities={visible_entities} unique_handles={} visible_unique={} missing={missing}\n  probes: foliage={foliage_probes} stick={stick_probes} total={probes_total}\n  lod_hosts={lod_hosts}",
			unique_handles.len(),
			visible_unique_handles.len(),
		);
		info!("{text}");
		ui::write_status(&mut status, text);
		commands.entity(entity).despawn();
	}
}

fn setup_lighting(mut commands: Commands) {
	commands.insert_resource(GlobalAmbientLight { brightness: 450.0, ..default() });
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
	playground: Res<PlaygroundConfig>,
) {
	let material = terrain_materials.add(DurhamTerrainShader::default());
	let (macro_seam_half_extents, macro_cell_min_size, macro_res_2) = match playground.coverage {
		TerrainCoverage::FinePatch => (Vec::new(), None, None),
		TerrainCoverage::PlayableWorld => {
			let s = TERRAIN_CELL_SIZE;
			let fine_half = WORLD_FINE_HALF_EXTENT_CELLS as f32 * s;
			let mid_half = fine_half + WORLD_OUTER_2X_ROWS as f32 * 2.0 * s;
			(vec![fine_half, mid_half], Some(2.0 * s), Some(2))
		}
	};
	commands.insert_resource(TerrainPresentationAssets {
		config: config.clone(),
		material,
		lod_bands: lod_bands_for(&playground),
		outer_add_walls: true,
		fine_grid_max_radius: Some(playground.terrain_radius),
		macro_seam_half_extents,
		macro_cell_min_size,
		macro_res_2,
	});
	commands.insert_resource(WaterPresentationAssets {
		material: water_materials.add(RefractionWater::default()),
	});
}

fn apply_commands(
	mut commands: Commands,
	mut playground: ResMut<PlaygroundConfig>,
	mut layout: ResMut<TerrainCellLayout>,
	mut terrain_assets: ResMut<TerrainPresentationAssets>,
	mut terrain_dirty: ResMut<TerrainPresentationDirty>,
	mut groves_dirty: ResMut<GrovesDirty>,
	mut status: Option<ResMut<GameCommandStatusText>>,
	grove: Query<(Entity, &RequestGrove)>,
	forest: Query<(Entity, &RequestForest)>,
	terrain_radius: Query<(Entity, &RequestTerrainRadius)>,
	grove_extent: Query<(Entity, &RequestGroveExtent)>,
	tile_radius: Query<(Entity, &RequestTileRadius)>,
	rebuild: Query<Entity, With<RequestRebuild>>,
) {
	for (entity, request) in &grove {
		playground.grove = request.0;
		playground.forest = None;
		groves_dirty.0 = true;
		ui::write_status(&mut status, format!("grove {}", request.0.label()));
		commands.entity(entity).despawn();
	}
	for (entity, request) in &forest {
		let spec = request.0;
		playground.forest = Some(spec);
		if playground.coverage == TerrainCoverage::FinePatch {
			let (_, generate_m) = stream_radii_m(spec.stream_radius);
			let needed = terrain_cells_for_generate_m(generate_m).max(1);
			if playground.terrain_radius < needed {
				playground.terrain_radius = needed;
				*layout = cell_layout(needed);
				terrain_assets.lod_bands = playground_lod_bands(needed);
				terrain_assets.fine_grid_max_radius = Some(needed);
				terrain_dirty.0 = true;
			}
		}
		groves_dirty.0 = true;
		let layering = spec.layering.map(|k| k.as_kebab()).unwrap_or("hopscotch");
		ui::write_status(&mut status, format!("forest {layering} r={}", spec.stream_radius));
		commands.entity(entity).despawn();
	}
	for (entity, request) in &terrain_radius {
		let cells = request.0.max(1);
		playground.terrain_radius = cells;
		if playground.coverage == TerrainCoverage::FinePatch {
			*layout = cell_layout(cells);
			terrain_assets.lod_bands = playground_lod_bands(cells);
			terrain_assets.fine_grid_max_radius = Some(cells);
			terrain_dirty.0 = true;
		}
		groves_dirty.0 = true;
		ui::write_status(&mut status, format!("terrain-radius {cells}"));
		commands.entity(entity).despawn();
	}
	for (entity, request) in &grove_extent {
		playground.grove_extent_xz = request.0.max(1.0);
		groves_dirty.0 = true;
		ui::write_status(&mut status, format!("grove-extent {}", playground.grove_extent_xz));
		commands.entity(entity).despawn();
	}
	for (entity, request) in &tile_radius {
		playground.tile_radius = request.0.max(0);
		groves_dirty.0 = true;
		ui::write_status(&mut status, format!("tile-radius {}", playground.tile_radius));
		commands.entity(entity).despawn();
	}
	for entity in &rebuild {
		groves_dirty.0 = true;
		ui::write_status(&mut status, "rebuild");
		commands.entity(entity).despawn();
	}
}

fn apply_mode_commands(
	mut commands: Commands,
	mut mode: ResMut<PlaygroundMode>,
	mut status: Option<ResMut<GameCommandStatusText>>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	store: Res<TerrainEntryStore>,
	free: Query<Entity, With<RequestModeFree>>,
	character: Query<Entity, With<RequestModeCharacter>>,
	mut players: Query<(Entity, &mut Transform, &mut LinearVelocity), With<Player>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), (With<Camera3d>, Without<Player>)>,
) {
	for entity in &free {
		*mode = PlaygroundMode::Free;
		ui::write_status(&mut status, "mode free");
		if let Ok((mut cam_t, mut controller)) = cameras.single_mut() {
			refocus_camera_on_elevation(
				&layout,
				surface_or_hold(&layout, &store, &base.0),
				&mut cam_t,
				&mut controller,
			);
		}
		commands.entity(entity).despawn();
	}

	for entity in &character {
		*mode = PlaygroundMode::Character;
		ui::write_status(&mut status, "mode character — WASD move, mouse look, Space jump");
		if let Ok((player, mut transform, mut velocity)) = players.single_mut() {
			let center = layout.region_center_xz();
			if let Some(elevation) = store.composed_height_at(&layout, center.x, center.z) {
				respawn_player_on_layout(&layout, elevation, &mut transform, &mut velocity);
			}
			commands.entity(player).insert(AwaitingTerrainSurface);
		}
		commands.entity(entity).despawn();
	}
}

fn generate_cells(
	mut commands: Commands,
	mut index: AvianTerrainIndex,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut world_base: ResMut<WorldBaseTerrain>,
	mode: Res<PlaygroundMode>,
	mut cameras: Query<(&mut Transform, &mut CameraController), (With<Camera3d>, Without<Player>)>,
	mut players: Query<(Entity, &mut Transform, &mut LinearVelocity), With<Player>>,
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
	info!("generated terrain_cells={} water_cells={}", terrains.len(), waters.len());

	if let Some(base) = index.base_noise() {
		world_base.0 = base.clone();
	}

	if let Ok((player, mut transform, mut velocity)) = players.single_mut() {
		let center = layout.region_center_xz();
		if let Some(elevation) = index.composed_height_at(center.x, center.z) {
			respawn_player_on_layout(&layout, elevation, &mut transform, &mut velocity);
		}
		commands.entity(player).insert(AwaitingTerrainSurface);
	}

	if *mode == PlaygroundMode::Free {
		if let Ok((mut transform, mut controller)) = cameras.single_mut() {
			let center = layout.region_center_xz();
			let elevation = index
				.composed_height_at(center.x, center.z)
				.unwrap_or_else(|| holding_elevation(&world_base.0, center.x, center.z));
			refocus_camera_on_elevation(&layout, elevation, &mut transform, &mut controller);
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

	terrain_presenter.clear_presented();
	water_presenter.clear_presented();

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
	let water_view = WaterStoreView::new(&store, &layout);
	RegionPresenter::<Water, _>::present(&mut water_presenter, &water_view, region, &lod_ref);
	pending.0 = false;
}

fn spawn_groves(
	mut commands: Commands,
	config: Res<PlaygroundConfig>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	mut dirty: ResMut<GrovesDirty>,
	pending: Res<TerrainPresentPending>,
	terrain_dirty: Res<TerrainPresentationDirty>,
	roots: Query<Entity, With<GroveRoot>>,
) {
	if !dirty.0 || pending.0 || terrain_dirty.0 {
		return;
	}

	for entity in &roots {
		commands.entity(entity).despawn();
	}

	if config.forest.is_some() {
		info!("forest stream on; tiled groves cleared");
		dirty.0 = false;
		return;
	}

	let n = spawn_tiled_groves(&mut commands, &config, &store, &layout, &base.0);
	info!(
		"spawned {} grove hosts ({} {}m tiles, r={})",
		n,
		config.grove.label(),
		config.grove_extent_xz,
		config.tile_radius
	);
	dirty.0 = false;
}

fn sync_pad_gameplay(
	focus: Option<Res<TextEntryFocus>>,
	blocked: Option<Res<TextEntryBlocked>>,
	mut enabled: ResMut<PadGameplayEnabled>,
) {
	let text = focus.is_some_and(|focus| focus.0) || blocked.is_some_and(|blocked| blocked.0);
	enabled.0 = !text;
}

#[cfg(test)]
mod tests {
	use super::*;
	use durham_terrain_models::origin_cell_ids_for_layout;

	#[test]
	fn world_defaults_keep_grove_fill_at_one_kilometre() {
		let spec = PlaygroundConfig::world_defaults().forest.expect("forest on");
		assert_eq!(spec.stream_radius, 1);
		assert_eq!(stream_radii_m(1), (1_000.0, 3_000.0));
	}

	#[test]
	fn world_fine_grid_stays_at_sixteen_cells() {
		assert_eq!(WORLD_FINE_HALF_EXTENT_CELLS, 16);
		assert!(!world_lod_bands().iter().any(|band| band.max_radius_cells > 16));
	}

	#[test]
	fn world_origin_cells_stay_on_fine_disk_plus_macro_rings() {
		let layout = world_cell_layout();
		let ids = origin_cell_ids_for_layout(&layout, layout.request_region());
		assert_eq!(ids.len(), 32 * 32 + 144 + 44);
	}

	#[test]
	fn world_macro_rings_stay_inside_seven_km() {
		let s = TERRAIN_CELL_SIZE;
		let fine = WORLD_FINE_HALF_EXTENT_CELLS as f32 * s;
		let mid = fine + WORLD_OUTER_2X_ROWS as f32 * 2.0 * s;
		let outer = mid + WORLD_OUTER_4X_ROWS as f32 * 4.0 * s;
		assert!(outer < 7_000.0, "playable half-extent {outer}");
	}
}
