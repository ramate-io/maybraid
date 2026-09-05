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

pub use bump_out::{
	bump_out_from_cell, bump_out_noise, fine_terrain_for, register_bump_out_lod, terrain_chunk_ref,
	CanopyBumpOutPresenterState, DurhamCanopyBumpOutPresenter, WorldTerrainBuilder,
};
pub use camera::CameraController;
pub use character::{CharacterSpecies, PlayerVisual, RequestSetCharacter};
pub use chico_sbs_trees_playground::forest_stream::ForestStreamSpec;
pub use commands::{GroveKind, PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use diagnostics::{PlaygroundDiag, PlaygroundTimingPlugin, RequestFpsToggle};
pub use forest::DurhamForestPresenter;
pub use game_commands::command::PendingStartupCommand;
pub use groves::{
	DurhamGroveSample, DurhamGroveTerrainCache, OwnedDurhamTerrain, StoredDurhamTerrain,
};
pub use material_lib::{
	init_vegetation_on_terrain_material_caches, VegetationOnTerrainMaterialLib,
	VegetationOnTerrainMaterialRefPlugin,
};
pub use player::{
	CharacterCameraFollowEnabled, CharacterLocomotion, MoveWish, MovementAction,
	PadMovementEnabled, Player, PlayerCapsule, PlayerControlSystems, PlayerPhysicsEnabled,
	PlayerPlugin, PlayerRespawn, PlaygroundMode, SpawnTerrainReady,
};

use avian3d::prelude::LinearVelocity;
use bevy::camera::visibility::VisibilitySystems;
use bevy::log::info_span;
use bevy::math::bounding::Aabb3d;
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bump_out::stream_canopy_bump_outs;
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
	origin_cell_ids_for_layout, AvianTerrainIndex, BaseTerrainNoise, ComposedWater,
	DurhamTerrainModelsPlugin, Terrain, TerrainBackgroundRegionPresenter, TerrainCellLayout,
	TerrainCellRing, TerrainConfig, TerrainEntryStore, TerrainFarRegionPresenter,
	TerrainGenerationResult, TerrainMeshBuilder, TerrainMeshLodBand, TerrainNearRegionPresenter,
	TerrainPresentationAssets, TerrainRegionPresenter, TerrainStoreView, Water,
	WaterPresentationAssets, WaterRegionPresenter, WaterStoreView, TERRAIN_CELL_SIZE,
};
use forest::stream_durham_forest;
use futures::FutureExt;
use game_commands::command::{
	capture_command_line_input, GameCommandPlugin, TextEntryBlocked, TextEntryFocus,
};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText};
use groves::{spawn_tiled_groves, GroveRoot};
use lod::gen::{Id, LodGenerateKeepRegion, LodGenerateRegion, RegionPresenter};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion};
use lod::{
	LodGenerateRegionPlugin, LodGenerateSystems, LodPresentRegionPlugin, LodPresentSystems,
	LodRefreshRegions, LodRefreshRegionsStatus, LodSceneHost, LodSceneRefreshRegionPlugin,
	LodViewer,
};
use lod_avian::AvianLodSceneRefreshPlugin;
use lod_first_load::{FirstLoadActivity, FirstLoadPermit};
use maybraid_input::{PadGameplayEnabled, VirtualPadPlugin, VirtualPadSystems};
use pitch::{apply_avian_terrain_pitch, sync_suspend_terrain_pitch};
use player::{
	holding_elevation, respawn_player_on_layout, snap_player_to_composed_surface,
	AwaitingTerrainSurface,
};
use render_item::mesh::handle::{EnforceCachingPlugin, EnforcedCaches};
use std::collections::{HashSet, VecDeque};
use std::f32::consts::PI;
use terrain_chunk_ref::{TerrainChunkRefCache, TerrainChunkRefPlugin};

const DEFAULT_TERRAIN_RADIUS: i32 = 2;
const DEFAULT_TILE_RADIUS: i32 = 1;

/// Near-stream High half-extent (8 × 160 m = 1.28 km).
const WORLD_FINE_HALF_EXTENT_CELLS: i32 = 8;
/// Near 160 m cells end on a boundary shared by the 320 m grid.
const WORLD_TERRAIN_NEAR_RADIUS_M: f32 = 8.0 * TERRAIN_CELL_SIZE;
/// Far 320 m cells end on a boundary shared by the 640 m grid.
const WORLD_TERRAIN_FAR_RADIUS_M: f32 = 16.0 * TERRAIN_CELL_SIZE;
/// Background 640 m cells provide the horizon out to the existing world edge.
const WORLD_TERRAIN_BACKGROUND_RADIUS_M: f32 = 24.0 * TERRAIN_CELL_SIZE;
/// Keep one near-band width of empty hosts around each visible band.
const WORLD_TERRAIN_CULL_MARGIN_M: f32 = WORLD_TERRAIN_NEAR_RADIUS_M;
const WORLD_TERRAIN_STREAM_EDGE_M: f32 =
	2.0 * (WORLD_TERRAIN_BACKGROUND_RADIUS_M + WORLD_TERRAIN_CULL_MARGIN_M);
/// Keep annulus boundaries aligned to the coarsest (640 m) global grid.
const WORLD_TERRAIN_PRESENT_STEP_M: f32 = 4.0 * TERRAIN_CELL_SIZE;
/// The retention width permits less frequent semantic regeneration.
const WORLD_TERRAIN_GENERATE_STEP_M: f32 = WORLD_TERRAIN_NEAR_RADIUS_M;
const WORLD_TERRAIN_QUERY_MIN_Y: f32 = -8_000.0;
const WORLD_TERRAIN_QUERY_MAX_Y: f32 = 8_000.0;
const INITIAL_TERRAIN_GENERATION_CELLS: usize = 4;
const TERRAIN_GENERATION_BATCH_CELLS: usize = 12;

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
	vec![TerrainMeshLodBand { max_radius_cells: WORLD_FINE_HALF_EXTENT_CELLS, res_2: 5 }]
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
	layout.outer_rings.clear();
	layout.stream_rings = vec![
		TerrainCellRing {
			cell_size: TERRAIN_CELL_SIZE,
			anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
			high_inner_radius: 0.0,
			high_outer_radius: WORLD_TERRAIN_NEAR_RADIUS_M,
			cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
		},
		TerrainCellRing {
			cell_size: 2.0 * TERRAIN_CELL_SIZE,
			anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
			high_inner_radius: WORLD_TERRAIN_NEAR_RADIUS_M,
			high_outer_radius: WORLD_TERRAIN_FAR_RADIUS_M,
			cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
		},
		TerrainCellRing {
			cell_size: 4.0 * TERRAIN_CELL_SIZE,
			anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
			high_inner_radius: WORLD_TERRAIN_FAR_RADIUS_M,
			high_outer_radius: WORLD_TERRAIN_BACKGROUND_RADIUS_M,
			cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
		},
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

/// Whether terrain and its dependent vegetation streams may advance.
///
/// Playgrounds default this on. The game shell turns it on when Discovery
/// enters its loading state, so the home menu does not eagerly build the world.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TerrainStreamingEnabled(pub bool);

impl Default for TerrainStreamingEnabled {
	fn default() -> Self {
		Self(true)
	}
}

pub fn terrain_streaming_enabled(enabled: Res<TerrainStreamingEnabled>) -> bool {
	enabled.0
}

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

#[derive(Resource, Default)]
struct TerrainGenerationTask {
	task: Option<Task<CompletedTerrainGeneration>>,
}

struct CompletedTerrainGeneration {
	result: TerrainGenerationResult,
	region: Aabb3d,
	_permit: Option<FirstLoadPermit>,
}

#[derive(Debug, Clone, Copy, Default)]
struct TerrainLodChannel;

#[derive(Debug, Clone, Copy, Default)]
struct TerrainSceneRefreshChannel;

#[derive(Resource, Debug, Clone, Copy, Default)]
struct TerrainGenerateRing;

#[derive(Resource, Debug, Clone, Copy, Default)]
struct TerrainPresentRing;

#[derive(Resource, Default)]
struct TerrainGenerationPlan {
	region: Option<Aabb3d>,
	batches: VecDeque<Vec<Id>>,
	desired: HashSet<Id>,
	retire_stale: bool,
	terrain_cells: usize,
	water_cells: usize,
}

impl TerrainGenerationPlan {
	fn rebuild(&mut self, layout: &TerrainCellLayout, region: Aabb3d) {
		let anchor = (Vec3::from(region.min) + Vec3::from(region.max)) * 0.5;
		let mut ids: Vec<_> = origin_cell_ids_for_layout(layout, region)
			.into_iter()
			.map(|original| original.0)
			.collect();
		ids.sort_by(|a, b| {
			let rank = |id: &Id| {
				let Some(bounds) = id.origin_cell_bounds() else {
					return (u8::MAX, f32::MAX);
				};
				let center = (Vec3::from(bounds.min) + Vec3::from(bounds.max)) * 0.5;
				let cell_size = bounds.max.x - bounds.min.x;
				let Some((scale, ring)) = layout
					.stream_rings
					.iter()
					.copied()
					.enumerate()
					.find(|(_, ring)| (ring.cell_size - cell_size).abs() < 1e-3)
				else {
					return (u8::MAX, f32::MAX);
				};
				let visible = ring.level_for(center, anchor) == lod::LodSceneLevel::High;
				let stage = scale as u8 + if visible { 0 } else { layout.stream_rings.len() as u8 };
				let delta = center - anchor;
				(stage, delta.x * delta.x + delta.z * delta.z)
			};
			rank(a)
				.partial_cmp(&rank(b))
				.unwrap_or(std::cmp::Ordering::Equal)
				.then_with(|| a.cmp(b))
		});
		self.desired = ids.iter().copied().collect();
		self.batches.clear();
		let first = ids.len().min(INITIAL_TERRAIN_GENERATION_CELLS);
		if first != 0 {
			self.batches.push_back(ids[..first].to_vec());
		}
		self.batches.extend(
			ids[first..].chunks(TERRAIN_GENERATION_BATCH_CELLS).map(|chunk| chunk.to_vec()),
		);
		self.region = Some(region);
		self.retire_stale = true;
		self.terrain_cells = 0;
		self.water_cells = 0;
	}
}

fn terrain_ring_region(position: Vec3, edge: f32, step: f32) -> Aabb3d {
	let center =
		Vec3::new((position.x / step).round() * step, 0.0, (position.z / step).round() * step);
	let half_xz = edge * 0.5;
	Aabb3d::from_min_max(
		Vec3::new(center.x - half_xz, WORLD_TERRAIN_QUERY_MIN_Y, center.z - half_xz),
		Vec3::new(center.x + half_xz, WORLD_TERRAIN_QUERY_MAX_Y, center.z + half_xz),
	)
}

fn terrain_ring_status(lod_ref: &LodRef, edge: f32, step: f32) -> LodRefreshRegionsStatus {
	let current = terrain_ring_region(lod_ref.current_transform.translation, edge, step);
	let previous = terrain_ring_region(lod_ref.previous_transform.translation, edge, step);
	if current == previous {
		LodRefreshRegionsStatus::Unchanged
	} else {
		LodRefreshRegionsStatus::Changed(current)
	}
}

impl LodRefreshRegions for TerrainGenerateRing {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		terrain_ring_status(lod_ref, WORLD_TERRAIN_STREAM_EDGE_M, WORLD_TERRAIN_GENERATE_STEP_M)
	}
}

impl LodRefreshRegions for TerrainPresentRing {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		terrain_ring_status(lod_ref, WORLD_TERRAIN_STREAM_EDGE_M, WORLD_TERRAIN_PRESENT_STEP_M)
	}
}

#[derive(Resource)]
struct GrovesDirty(bool);

pub struct VegetationOnTerrainPlugin {
	pub config: PlaygroundConfig,
	/// When false, the caller owns the command drawer / CLI.
	pub commands: bool,
	/// Register the plain Durham-backed forest presenter.
	pub register_forest_lod: bool,
	/// Register the plain Durham-backed canopy bump-out presenter.
	pub register_bump_out_lod: bool,
}

impl Default for VegetationOnTerrainPlugin {
	fn default() -> Self {
		Self {
			config: PlaygroundConfig::default(),
			commands: true,
			register_forest_lod: true,
			register_bump_out_lod: true,
		}
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
		app.add_plugins((
			LodGenerateRegionPlugin::<
				TerrainGenerateRing,
				With<LodViewer>,
				TerrainLodChannel,
			>::default(),
			LodPresentRegionPlugin::<
				TerrainPresentRing,
				With<LodViewer>,
				TerrainLodChannel,
			>::default(),
			LodSceneRefreshRegionPlugin::<
				TerrainPresentRing,
				With<LodViewer>,
				TerrainSceneRefreshChannel,
			>::default(),
			AvianLodSceneRefreshPlugin::<
				Terrain,
				TerrainSceneRefreshChannel,
				With<LodViewer>,
			>::default(),
		));
		if !app.is_plugin_added::<VegetationOnTerrainMaterialRefPlugin>() {
			app.add_plugins(VegetationOnTerrainMaterialRefPlugin);
		}
		if self.register_forest_lod {
			register_forest_lod::<DurhamForestPresenter>(app);
		}
		if self.register_bump_out_lod {
			register_bump_out_lod::<DurhamCanopyBumpOutPresenter>(app);
		}
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
			.init_resource::<TerrainStreamingEnabled>()
			.insert_resource(playground.clone())
			.insert_resource(layout_for(&playground))
			.insert_resource(TerrainPresentationDirty(true))
			.init_resource::<TerrainPresentPending>()
			.init_resource::<TerrainGenerationTask>()
			.init_resource::<TerrainGenerationPlan>()
			.insert_resource(TerrainGenerateRing)
			.insert_resource(TerrainPresentRing)
			.insert_resource(GrovesDirty(true))
			.init_resource::<DurhamGroveTerrainCache>()
			.add_systems(Startup, (setup_camera, setup_lighting, setup_presentation_assets))
			.add_systems(PreUpdate, sync_pad_gameplay.before(VirtualPadSystems::Produce))
			.add_systems(Update, refresh_grove_terrain_cache.before(LodPresentSystems::Produce))
			.add_systems(PostUpdate, apply_mesh_stats.after(VisibilitySystems::CheckVisibility));
		if self.commands {
			app.add_systems(
				Update,
				(
					release_modifiers_on_focus_change.before(camera_controller),
					camera_controller,
					apply_commands.after(capture_command_line_input::<PlaygroundCommand>),
					initialize_terrain_lod_regions
						.after(apply_commands)
						.before(generate_cells)
						.run_if(terrain_streaming_enabled),
					generate_cells.after(apply_commands).run_if(terrain_streaming_enabled),
					present_cells.after(generate_cells).run_if(terrain_streaming_enabled),
					spawn_groves.after(present_cells).run_if(terrain_streaming_enabled),
					stream_durham_forest
						.after(apply_commands)
						.before(LodGenerateSystems::Produce)
						.before(LodPresentSystems::Produce)
						.run_if(terrain_streaming_enabled),
					stream_canopy_bump_outs
						.after(stream_durham_forest)
						.before(LodGenerateSystems::Produce)
						.before(LodPresentSystems::Produce)
						.run_if(terrain_streaming_enabled),
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
					initialize_terrain_lod_regions
						.before(generate_cells)
						.run_if(terrain_streaming_enabled),
					generate_cells.run_if(terrain_streaming_enabled),
					present_cells.after(generate_cells).run_if(terrain_streaming_enabled),
					spawn_groves.after(present_cells).run_if(terrain_streaming_enabled),
					stream_durham_forest
						.before(LodGenerateSystems::Produce)
						.before(LodPresentSystems::Produce)
						.run_if(terrain_streaming_enabled),
					stream_canopy_bump_outs
						.after(stream_durham_forest)
						.before(LodGenerateSystems::Produce)
						.before(LodPresentSystems::Produce)
						.run_if(terrain_streaming_enabled),
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

fn refresh_grove_terrain_cache(
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	mut cache: ResMut<DurhamGroveTerrainCache>,
) {
	if cache.terrain.is_some() && !store.is_changed() && !layout.is_changed() && !base.is_changed()
	{
		return;
	}
	cache.terrain =
		Some(OwnedDurhamTerrain::new(store.height_snapshot(), layout.clone(), base.0.clone()));
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
		TerrainCoverage::PlayableWorld => (Vec::new(), Some(2.0 * TERRAIN_CELL_SIZE), Some(5)),
	};
	commands.insert_resource(TerrainPresentationAssets {
		config: config.clone(),
		material,
		lod_bands: lod_bands_for(&playground),
		// Moving coarse rings must not emit full-depth walls on every cell face.
		outer_add_walls: playground.coverage == TerrainCoverage::FinePatch,
		fine_grid_max_radius: (playground.coverage == TerrainCoverage::FinePatch)
			.then_some(playground.terrain_radius),
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

fn initialize_terrain_lod_regions(
	config: Res<PlaygroundConfig>,
	players: Query<&Transform, (With<Player>, Without<Camera3d>)>,
	cameras: Query<&Transform, (With<Camera3d>, Without<Player>)>,
	mut generate_keep: ResMut<LodGenerateKeepRegion<TerrainLodChannel>>,
	mut present_keep: ResMut<LodPresentKeepRegion<TerrainLodChannel>>,
	mut generate: MessageWriter<LodGenerateRegion<TerrainLodChannel>>,
	mut present: MessageWriter<LodPresentRegion<TerrainLodChannel>>,
) {
	if config.coverage != TerrainCoverage::PlayableWorld
		|| (generate_keep.region.is_some() && present_keep.region.is_some())
	{
		return;
	}
	let position = players
		.single()
		.map(|transform| transform.translation)
		.or_else(|_| cameras.single().map(|transform| transform.translation))
		.unwrap_or(Vec3::ZERO);
	if generate_keep.region.is_none() {
		let region = terrain_ring_region(
			position,
			WORLD_TERRAIN_STREAM_EDGE_M,
			WORLD_TERRAIN_GENERATE_STEP_M,
		);
		generate_keep.region = Some(region);
		generate.write(LodGenerateRegion::new(region));
	}
	if present_keep.region.is_none() {
		let region = terrain_ring_region(
			position,
			WORLD_TERRAIN_STREAM_EDGE_M,
			WORLD_TERRAIN_PRESENT_STEP_M,
		);
		present_keep.region = Some(region);
		present.write(LodPresentRegion::new(region));
	}
}

fn generate_cells(
	mut index: AvianTerrainIndex,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut generation: ResMut<TerrainGenerationTask>,
	mut plan: ResMut<TerrainGenerationPlan>,
	mut generated_regions: MessageReader<LodGenerateRegion<TerrainLodChannel>>,
	generate_keep: Res<LodGenerateKeepRegion<TerrainLodChannel>>,
	activity: Option<Res<FirstLoadActivity>>,
	mut world_base: ResMut<WorldBaseTerrain>,
	config: Res<PlaygroundConfig>,
	mode: Res<PlaygroundMode>,
	mut cameras: Query<(&mut Transform, &mut CameraController), (With<Camera3d>, Without<Player>)>,
) {
	let streamed = config.coverage == TerrainCoverage::PlayableWorld;
	let region_changed = generated_regions.read().next().is_some();
	if dirty.0 {
		index.clear();
		plan.region = None;
		dirty.0 = false;
	}
	let requested_region =
		if streamed { generate_keep.region } else { Some(index.layout().request_region()) };
	if (region_changed || plan.region.is_none())
		&& requested_region.is_some_and(|region| plan.region != Some(region))
	{
		plan.rebuild(
			index.layout(),
			requested_region.unwrap_or_else(|| index.layout().request_region()),
		);
	}

	if let Some(task) = generation.task.as_mut() {
		let Some(completed) = (&mut *task).now_or_never() else {
			return;
		};
		generation.task = None;
		if plan.region == Some(completed.region) {
			let initial_batch = plan.terrain_cells == 0;
			plan.terrain_cells += completed.result.terrain_cells;
			plan.water_cells += completed.result.water_cells;
			index.apply_generation_batch(completed.result);
			if initial_batch {
				info!(
					"generated initial terrain batch terrain_cells={} water_cells={}",
					plan.terrain_cells, plan.water_cells
				);
			}
			if let Some(base) = index.base_noise() {
				world_base.0 = base.clone();
			}
			pending.0 = true;
		}
	}

	let Some(region) = plan.region else {
		return;
	};
	let Some(ids) = plan.batches.pop_front() else {
		if plan.retire_stale {
			index.retain_generation_ids(&plan.desired);
			plan.retire_stale = false;
			pending.0 = true;
			info!(
				"generated terrain_cells={} water_cells={}",
				plan.terrain_cells, plan.water_cells
			);
			if !streamed && *mode == PlaygroundMode::Free {
				let layout = index.layout().clone();
				if let Ok((mut transform, mut controller)) = cameras.single_mut() {
					let center = layout.region_center_xz();
					let elevation = index
						.composed_height_at(center.x, center.z)
						.unwrap_or_else(|| holding_elevation(&world_base.0, center.x, center.z));
					refocus_camera_on_elevation(
						&layout,
						elevation,
						&mut transform,
						&mut controller,
					);
				}
			}
		}
		return;
	};
	let input = index.generation_input();
	// The first visible near batch is sufficient to hand off from the loading
	// screen; retention and distant rings continue streaming afterward.
	let permit = (plan.terrain_cells == 0)
		.then(|| activity.as_ref().map(|activity| activity.begin()))
		.flatten();
	generation.task = Some(AsyncComputeTaskPool::get().spawn(async move {
		let _span = info_span!("durham_terrain_generation").entered();
		CompletedTerrainGeneration { result: input.generate_ids(ids), region, _permit: permit }
	}));
	pending.0 = true;
}

fn present_cells(
	mut terrain_presenter: TerrainRegionPresenter,
	mut near_presenter: TerrainNearRegionPresenter,
	mut far_presenter: TerrainFarRegionPresenter,
	mut background_presenter: TerrainBackgroundRegionPresenter,
	mut water_presenter: WaterRegionPresenter,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	config: Res<PlaygroundConfig>,
	mut pending: ResMut<TerrainPresentPending>,
	generation: Res<TerrainGenerationTask>,
	present_keep: Res<LodPresentKeepRegion<TerrainLodChannel>>,
	mut present_regions: MessageReader<LodPresentRegion<TerrainLodChannel>>,
	viewers: Query<&Transform, With<LodViewer>>,
) {
	let streamed = config.coverage == TerrainCoverage::PlayableWorld;
	let region_changed = present_regions.read().next().is_some();
	if !streamed {
		if !pending.0 || generation.task.is_some() {
			return;
		}
		terrain_presenter.clear_presented();
		water_presenter.clear_presented();
	} else if !pending.0 && !region_changed {
		return;
	}
	if store.is_empty() {
		return;
	}
	let region = if streamed {
		let Some(region) = present_keep.region else {
			return;
		};
		region
	} else {
		layout.request_region()
	};
	let viewer = viewers.single().copied().unwrap_or(Transform::IDENTITY);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &viewer,
		current_transform: &viewer,
		bounds: &region,
	};
	let terrain_view = TerrainStoreView::new(&store, &layout);
	if streamed {
		near_presenter.present(&store, region, &lod_ref);
		far_presenter.present(&store, region, &lod_ref);
		background_presenter.present(&store, region, &lod_ref);
	} else {
		RegionPresenter::<Terrain, _>::present(
			&mut terrain_presenter,
			&terrain_view,
			region,
			&lod_ref,
		);
	}
	if !streamed {
		let water_view = WaterStoreView::new(&store, &layout);
		RegionPresenter::<Water, _>::present(&mut water_presenter, &water_view, region, &lod_ref);
	}
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
	if config.forest.is_some() {
		dirty.0 = false;
		return;
	}
	if !dirty.0 || pending.0 || terrain_dirty.0 {
		return;
	}

	for entity in &roots {
		commands.entity(entity).despawn();
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
	fn world_near_stream_stays_at_eight_cells() {
		assert_eq!(WORLD_FINE_HALF_EXTENT_CELLS, 8);
		assert_eq!(world_lod_bands(), vec![TerrainMeshLodBand { max_radius_cells: 8, res_2: 5 }]);
	}

	#[test]
	fn world_origin_cells_use_three_moving_scales() {
		let layout = world_cell_layout();
		assert_eq!(layout.stream_rings.len(), 3);
		assert_eq!(layout.stream_rings[0].cell_size, TERRAIN_CELL_SIZE);
		assert_eq!(layout.stream_rings[1].cell_size, 2.0 * TERRAIN_CELL_SIZE);
		assert_eq!(layout.stream_rings[2].cell_size, 4.0 * TERRAIN_CELL_SIZE);
		let ids = origin_cell_ids_for_layout(&layout, layout.request_region());
		assert!(ids.len() > 32 * 32);
	}

	#[test]
	fn world_stream_boundaries_align_all_three_grids() {
		assert_eq!(WORLD_TERRAIN_NEAR_RADIUS_M % (2.0 * TERRAIN_CELL_SIZE), 0.0);
		assert_eq!(WORLD_TERRAIN_FAR_RADIUS_M % (4.0 * TERRAIN_CELL_SIZE), 0.0);
		assert_eq!(WORLD_TERRAIN_BACKGROUND_RADIUS_M, 3_840.0);
	}

	#[test]
	fn world_generation_starts_with_visible_near_cells() {
		let layout = world_cell_layout();
		let region = terrain_ring_region(
			Vec3::ZERO,
			WORLD_TERRAIN_STREAM_EDGE_M,
			WORLD_TERRAIN_GENERATE_STEP_M,
		);
		let mut plan = TerrainGenerationPlan::default();
		plan.rebuild(&layout, region);
		let first = plan.batches.front().expect("non-empty world generation plan");
		assert_eq!(first.len(), INITIAL_TERRAIN_GENERATION_CELLS);
		for id in first {
			let bounds = id.origin_cell_bounds().expect("origin cell");
			assert!((bounds.max.x - bounds.min.x - TERRAIN_CELL_SIZE).abs() < 1e-3);
			let center = (Vec3::from(bounds.min) + Vec3::from(bounds.max)) * 0.5;
			assert_eq!(
				layout.stream_rings[0].level_for(center, Vec3::ZERO),
				lod::LodSceneLevel::High
			);
		}
	}

	#[test]
	fn world_producer_spans_supported_height() {
		let region = terrain_ring_region(
			Vec3::ZERO,
			WORLD_TERRAIN_STREAM_EDGE_M,
			WORLD_TERRAIN_GENERATE_STEP_M,
		);
		assert_eq!(region.min.y, WORLD_TERRAIN_QUERY_MIN_Y);
		assert_eq!(region.max.y, WORLD_TERRAIN_QUERY_MAX_Y);
	}
}
