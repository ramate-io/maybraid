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
use bevy::math::bounding::Aabb3d;
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
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
	origin_cell_ids_for_layout, BaseTerrainNoise, ComposedWater, DurhamTerrainModelsPlugin,
	TerrainCellLayout, TerrainCellRing, TerrainConfig, TerrainEntryStore, TerrainLodPlugin,
	TerrainMeshBuilder, TerrainMeshLodBand, TerrainPresentationAssets, WaterPresentationAssets,
	TERRAIN_CELL_SIZE,
};
use forest::stream_durham_forest;
use game_commands::command::{
	capture_command_line_input, GameCommandPlugin, TextEntryBlocked, TextEntryFocus,
};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText};
use groves::{spawn_tiled_groves, GroveRoot};
use lod::gen::LodGenerateKeepRegion;
use lod::lod_ref::LodRef;
use lod::presentation::LodPresentKeepRegion;
use lod::{
	LodCullRegionCursor, LodCullRegions, LodCullRegionsStatus, LodGenerateSystems,
	LodPresentSystems, LodRefreshRegions, LodRefreshRegionsStatus, LodSceneHost, LodViewer,
	OpenLattice,
};
use maybraid_input::{PadGameplayEnabled, VirtualPadPlugin, VirtualPadSystems};
use pitch::{apply_avian_terrain_pitch, sync_suspend_terrain_pitch};
use player::{respawn_player_on_layout, snap_player_to_composed_surface, AwaitingTerrainSurface};
use render_item::mesh::handle::{EnforceCachingPlugin, EnforcedCaches};
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
const WORLD_TERRAIN_QUERY_MIN_Y: f32 = -8_000.0;
const WORLD_TERRAIN_QUERY_MAX_Y: f32 = 8_000.0;

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
			res_2: 5,
			anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
			high_inner_radius: 0.0,
			high_outer_radius: WORLD_TERRAIN_NEAR_RADIUS_M,
			cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
		},
		TerrainCellRing {
			cell_size: 2.0 * TERRAIN_CELL_SIZE,
			res_2: 4,
			anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
			high_inner_radius: WORLD_TERRAIN_NEAR_RADIUS_M,
			high_outer_radius: WORLD_TERRAIN_FAR_RADIUS_M,
			cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
		},
		TerrainCellRing {
			cell_size: 4.0 * TERRAIN_CELL_SIZE,
			res_2: 3,
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

#[derive(Debug, Clone, Copy, Default)]
struct TerrainLodChannel;

#[derive(Resource, Debug, Clone, Copy)]
struct TerrainLodRegion {
	moving: bool,
	cull: OpenLattice,
}

impl Default for TerrainLodRegion {
	fn default() -> Self {
		Self {
			moving: true,
			cull: OpenLattice::new(
				WORLD_TERRAIN_STREAM_EDGE_M,
				WORLD_TERRAIN_STREAM_EDGE_M + 2.0 * WORLD_TERRAIN_PRESENT_STEP_M,
				WORLD_TERRAIN_PRESENT_STEP_M,
			),
		}
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

impl LodCullRegions for TerrainLodRegion {
	fn lod_cull_regions(
		&self,
		lod_refs: &[LodRef],
		cursor: &mut LodCullRegionCursor,
	) -> LodCullRegionsStatus {
		if !self.moving {
			return LodCullRegionsStatus::Unchanged;
		}
		self.cull.lod_cull_regions(lod_refs, cursor)
	}
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

impl LodRefreshRegions for TerrainLodRegion {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if self.moving {
			terrain_ring_status(lod_ref, WORLD_TERRAIN_STREAM_EDGE_M, WORLD_TERRAIN_PRESENT_STEP_M)
		} else {
			LodRefreshRegionsStatus::Unchanged
		}
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
		app.add_plugins(
			TerrainLodPlugin::<TerrainLodRegion, With<LodViewer>, TerrainLodChannel>::default(),
		);
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
			.insert_resource(TerrainLodRegion {
				moving: playground.coverage == TerrainCoverage::PlayableWorld,
				..default()
			})
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
						.before(LodGenerateSystems::Produce)
						.before(LodPresentSystems::Produce)
						.run_if(terrain_streaming_enabled),
					spawn_groves.after(LodPresentSystems::Drain).run_if(terrain_streaming_enabled),
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
						.after(LodPresentSystems::Drain)
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
						.before(LodGenerateSystems::Produce)
						.before(LodPresentSystems::Produce)
						.run_if(terrain_streaming_enabled),
					spawn_groves.after(LodPresentSystems::Drain).run_if(terrain_streaming_enabled),
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
						.after(LodPresentSystems::Drain)
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
		outer_add_walls: true,
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
	layout: Res<TerrainCellLayout>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	players: Query<&Transform, (With<Player>, Without<Camera3d>)>,
	cameras: Query<&Transform, (With<Camera3d>, Without<Player>)>,
	mut generate_keep: ResMut<LodGenerateKeepRegion<TerrainLodChannel>>,
	mut present_keep: ResMut<LodPresentKeepRegion<TerrainLodChannel>>,
) {
	if !dirty.0 && generate_keep.region.is_some() && present_keep.region.is_some() {
		return;
	}
	let region = match config.coverage {
		TerrainCoverage::FinePatch => layout.request_region(),
		TerrainCoverage::PlayableWorld => {
			let position = players
				.single()
				.map(|transform| transform.translation)
				.or_else(|_| cameras.single().map(|transform| transform.translation))
				.unwrap_or(Vec3::ZERO);
			terrain_ring_region(position, WORLD_TERRAIN_STREAM_EDGE_M, WORLD_TERRAIN_PRESENT_STEP_M)
		}
	};
	generate_keep.region = Some(region);
	present_keep.region = Some(region);
	dirty.0 = false;
}

fn spawn_groves(
	mut commands: Commands,
	config: Res<PlaygroundConfig>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	mut dirty: ResMut<GrovesDirty>,
	roots: Query<Entity, With<GroveRoot>>,
) {
	if config.forest.is_some() {
		dirty.0 = false;
		return;
	}
	if !dirty.0 {
		return;
	}
	let terrain_ready = origin_cell_ids_for_layout(&layout, layout.request_region())
		.into_iter()
		.all(|original| store.terrain(original.0).is_some());
	if !terrain_ready {
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
		assert_eq!(layout.stream_rings[0].res_2, 5);
		assert_eq!(layout.stream_rings[1].cell_size, 2.0 * TERRAIN_CELL_SIZE);
		assert_eq!(layout.stream_rings[1].res_2, 4);
		assert_eq!(layout.stream_rings[2].cell_size, 4.0 * TERRAIN_CELL_SIZE);
		assert_eq!(layout.stream_rings[2].res_2, 3);
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
	fn world_producer_spans_supported_height() {
		let region = terrain_ring_region(
			Vec3::ZERO,
			WORLD_TERRAIN_STREAM_EDGE_M,
			WORLD_TERRAIN_PRESENT_STEP_M,
		);
		assert_eq!(region.min.y, WORLD_TERRAIN_QUERY_MIN_Y);
		assert_eq!(region.max.y, WORLD_TERRAIN_QUERY_MAX_Y);
	}
}
