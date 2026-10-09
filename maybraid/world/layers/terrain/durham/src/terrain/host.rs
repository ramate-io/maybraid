//! World-facing streamed terrain: models, shaders, mesh caches, and fill present.
//!
//! Playable coverage is three scale streams (160 / 320 / 640 m) that follow the
//! viewer. Near cells use `res_2 = 5` and own collision; far and background are
//! render-only at `res_2 = 4` and `3`. Far / Background holes inset so Medium
//! overlaps the next-finer High rim. The [`TerrainWindow`] producer publishes
//! the layout's request region; `Terrain` and `Water` subscribe to it and
//! admit a bounded number of missing origin ids per frame. Playable visuals come from the urbanized
//! presenter. Generation runs on every coverage. Raw present is
//! [`crate::DurhamCells`], gated by the presenter subscription.

use bevy::ecs::system::SystemParam;
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use lod::gen::{Id, LodGenerateBudget, LodGenerated, Version};
use lod::hcsg::{
	ChannelTimeBudget, GenerateOn, GenerationProducer, HcsgSeedSystems, HcsgStorage, Seed,
};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use lod::LodViewer;
use render_item::mesh::handle::MeshFulfillBudget;
use std::collections::HashSet;
use terrain_shaders::{RefractionWater, TerrainShader, TerrainShaderPlugin};
use visual_geometry_core::{
	install_enforced_mesh_cache, share_terrain_chunk_refs, VisualGeometryCorePlugin,
};

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::cell::{
	universal_bounds, TerrainCellLayout, TerrainCellRing, TERRAIN_CELL_SIZE,
};
use crate::terrain::collider::{TerrainColliderEpoch, TerrainColliderSystems};
use crate::terrain::config::TerrainConfig;
use crate::terrain::index::{DurhamNodes, TerrainStorage};
use crate::terrain::presentation::{
	TerrainBackground, TerrainFar, TerrainMeshLodBand, TerrainNear, TerrainPresentationAssets,
	TerrainPresenterState, TerrainRegionPresenter, TerrainStreamPresenterState,
};
use crate::terrain::stamps::TerrainStampConfigs;
use crate::terrain::watersheds::WatershedConfigs;
use crate::water::{ComposedWater, Water, WaterPresentationAssets};
use crate::{DurhamTerrainModelsPlugin, Terrain, TerrainMeshBuilder};
use layer_stack::{mode_subscribed, LodPresentGateSync};
use lod::LodPresentGate;
use terrain_layer_model::{
	terrain_streaming, OnTerrain, TerrainExtent, TerrainLayerSystems, TerrainStreaming,
};
use terrain_layer_presentation::TerrainPresenter;

/// Composed Durham SDF / CpuShot terrain model.
pub struct Durham;

/// Raw Durham cells. Present while a mode is subscribed to `(OnTerrain<Durham>, Self)`.
pub struct DurhamCells;

impl TerrainPresenter<OnTerrain<Durham>> for DurhamCells {
	fn install(app: &mut App) {
		install_durham_presentation(app);
	}
}

/// Near-stream High half-extent (8 × 160 m = 1.28 km).
pub const WORLD_FINE_HALF_EXTENT_CELLS: i32 = 8;
/// 2× macro ring past the fine grid (FinePatch / nested-footprint layouts).
pub const WORLD_OUTER_2X_ROWS: i32 = 2;
/// 4× macro ring past the 2× ring.
pub const WORLD_OUTER_4X_ROWS: i32 = 1;
/// Near High outer radius.
const WORLD_TERRAIN_NEAR_RADIUS_M: f32 = 8.0 * TERRAIN_CELL_SIZE;
/// Far High outer radius (320 m cells).
const WORLD_TERRAIN_FAR_RADIUS_M: f32 = 16.0 * TERRAIN_CELL_SIZE;
/// Background High outer radius (640 m cells).
const WORLD_TERRAIN_BACKGROUND_RADIUS_M: f32 = 24.0 * TERRAIN_CELL_SIZE;
/// Far Medium starts this far inside the Near High disk (one Far cell).
/// Covers 640 m anchor snap plus half a 320 m tile so the hole is never empty.
const WORLD_TERRAIN_FAR_HOLE_INSET_M: f32 = 2.0 * TERRAIN_CELL_SIZE;
/// Background Medium starts this far inside the Far ring (one Background cell).
const WORLD_TERRAIN_BACKGROUND_HOLE_INSET_M: f32 = 4.0 * TERRAIN_CELL_SIZE;
/// Generate keep matches the drawn band. Do not use this as a draw overlap —
/// Near overflow is classified Medium and Near only draws High.
const WORLD_TERRAIN_CULL_MARGIN_M: f32 = 0.0;
/// Snap stream anchors to the 640 m lattice.
const WORLD_TERRAIN_PRESENT_STEP_M: f32 = 4.0 * TERRAIN_CELL_SIZE;
/// Sync Durham DAGs admitted per frame, nearest missing origin ids first.
const TERRAIN_ADMIT_PER_FRAME: u32 = 4;

/// Producer channel for Durham's terrain window: the [`TerrainCellLayout`]
/// request region around the viewer.
pub struct TerrainWindow;

/// Fine-only patch vs playable world extents (fine grid + macro rings).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerrainCoverage {
	#[default]
	FinePatch,
	PlayableWorld,
}

/// When true, [`produce_terrain_window`] keeps the current origin instead of
/// recentering on the viewer. A pinned fine patch uses this so the window stays put.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerrainLayoutPinned(pub bool);

/// Durham fill. Layout retargets run before [`Self::Generate`], which holds
/// [`Self::Window`] then [`Self::Subscribe`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainFillSystems {
	Generate,
	/// [`TerrainWindow`] producer: rebuilds, recenters, seeds the layout.
	Window,
	/// `Terrain` / `Water` subscribers on [`TerrainWindow`].
	Subscribe,
}

/// Base noise used for camera height before (and alongside) generation.
#[derive(Resource)]
pub struct WorldBaseTerrain(pub BaseTerrainNoise);

/// When true, fill should clear and rebuild (playground radius / seed commands).
#[derive(Resource, Default)]
pub struct TerrainPresentationDirty(pub bool);

fn extent_from_layout<M: Send + Sync + 'static>(layout: &TerrainCellLayout) -> TerrainExtent<M> {
	if layout.is_streamed() {
		TerrainExtent::streamed(layout.presentation_region())
	} else {
		TerrainExtent::pinned(layout.presentation_region())
	}
}

fn write_durham_extent(layout: Res<TerrainCellLayout>, mut extent: ResMut<TerrainExtent<Durham>>) {
	if !layout.is_changed() {
		return;
	}
	*extent = extent_from_layout::<Durham>(&layout);
}

/// When true, fill meshes have generated and present is still owed.
#[derive(Resource, Default)]
pub struct TerrainPresentPending(pub bool);

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
	layout.stream_rings.clear();
	layout
}

/// Playable near stream: 160 m cells that own collision.
pub const WORLD_NEAR_RING: TerrainCellRing = TerrainCellRing {
	cell_size: TERRAIN_CELL_SIZE,
	res_2: 5,
	anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
	high_inner_radius: 0.0,
	high_outer_radius: WORLD_TERRAIN_NEAR_RADIUS_M,
	cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
};

/// Playable far stream: 320 m cells around the near disk.
pub const WORLD_FAR_RING: TerrainCellRing = TerrainCellRing {
	cell_size: 2.0 * TERRAIN_CELL_SIZE,
	res_2: 4,
	anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
	high_inner_radius: WORLD_TERRAIN_NEAR_RADIUS_M - WORLD_TERRAIN_FAR_HOLE_INSET_M,
	high_outer_radius: WORLD_TERRAIN_FAR_RADIUS_M,
	cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
};

/// Playable background stream: 640 m cells around the far ring.
pub const WORLD_BACKGROUND_RING: TerrainCellRing = TerrainCellRing {
	cell_size: 4.0 * TERRAIN_CELL_SIZE,
	res_2: 3,
	anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
	high_inner_radius: WORLD_TERRAIN_FAR_RADIUS_M - WORLD_TERRAIN_BACKGROUND_HOLE_INSET_M,
	high_outer_radius: WORLD_TERRAIN_BACKGROUND_RADIUS_M,
	cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
};

fn world_cell_layout() -> TerrainCellLayout {
	let mut layout = TerrainCellLayout::default();
	layout.origin = IVec2::new(-WORLD_FINE_HALF_EXTENT_CELLS, -WORLD_FINE_HALF_EXTENT_CELLS);
	let n = (2 * WORLD_FINE_HALF_EXTENT_CELLS) as u32;
	layout.extents = UVec2::new(n, n);
	layout.outer_rings.clear();
	layout.stream_rings = vec![WORLD_NEAR_RING, WORLD_FAR_RING, WORLD_BACKGROUND_RING];
	layout
}

/// Playable-world rings (near / far / background).
pub fn playable_world_cell_layout() -> TerrainCellLayout {
	world_cell_layout()
}

/// Fine-only grid whose minimum corner is `origin`.
///
/// Extents are `2 * half_extent` cells on each axis (at least one cell).
/// No stream rings.
pub fn fine_patch_cell_layout(half_extent: i32, origin: IVec2) -> TerrainCellLayout {
	let mut layout = cell_layout(half_extent);
	layout.origin = origin;
	layout
}

fn lod_bands_for(coverage: TerrainCoverage, terrain_radius: i32) -> Vec<TerrainMeshLodBand> {
	match coverage {
		TerrainCoverage::FinePatch => playground_lod_bands(terrain_radius),
		TerrainCoverage::PlayableWorld => world_lod_bands(),
	}
}

/// Point presentation assets at a coverage after a live session retarget.
pub fn retarget_presentation_assets(
	assets: &mut TerrainPresentationAssets,
	coverage: TerrainCoverage,
	terrain_radius: i32,
) {
	let (macro_seam_half_extents, macro_cell_min_size, macro_res_2) = match coverage {
		TerrainCoverage::FinePatch => (Vec::new(), None, None),
		TerrainCoverage::PlayableWorld => (
			vec![WORLD_TERRAIN_NEAR_RADIUS_M, WORLD_TERRAIN_FAR_RADIUS_M],
			Some(2.0 * TERRAIN_CELL_SIZE),
			Some(3),
		),
	};
	assets.lod_bands = lod_bands_for(coverage, terrain_radius);
	assets.fine_grid_max_radius = Some(terrain_radius);
	assets.macro_seam_half_extents = macro_seam_half_extents;
	assets.macro_cell_min_size = macro_cell_min_size;
	assets.macro_res_2 = macro_res_2;
}

/// Layout, coverage, pin, dirty, pending, and presentation assets for a retarget.
///
/// Assets are created on Startup; a first-frame enter can run before they exist.
#[derive(SystemParam)]
pub struct TerrainRetarget<'w> {
	layout: ResMut<'w, TerrainCellLayout>,
	coverage: ResMut<'w, TerrainCoverage>,
	pinned: ResMut<'w, TerrainLayoutPinned>,
	dirty: ResMut<'w, TerrainPresentationDirty>,
	pending: ResMut<'w, TerrainPresentPending>,
	assets: Option<ResMut<'w, TerrainPresentationAssets>>,
}

impl TerrainRetarget<'_> {
	pub fn coverage(&self) -> TerrainCoverage {
		*self.coverage
	}

	pub fn layout(&self) -> &TerrainCellLayout {
		&self.layout
	}

	pub fn apply(
		&mut self,
		layout: TerrainCellLayout,
		coverage: TerrainCoverage,
		terrain_radius: i32,
		pin: bool,
	) {
		*self.layout = layout;
		*self.coverage = coverage;
		self.pinned.0 = pin;
		self.dirty.0 = true;
		self.pending.0 = true;
		if let Some(assets) = self.assets.as_mut() {
			retarget_presentation_assets(assets, coverage, terrain_radius);
		}
	}
}

/// Generation half of the old Durham terrain plugin: models, shaders, mesh
/// caches, layout, the [`TerrainWindow`] producer, and its subscribers.
///
/// `setup_presentation_assets` stays here. Generation reads the seeded
/// [`TerrainPresentationAssets`] and [`WaterPresentationAssets`] while filling
/// cells, and a live session retargets the terrain assets even when raw present is off.
pub(crate) fn install_durham_generation(app: &mut App) {
	if !app.is_plugin_added::<VisualGeometryCorePlugin>() {
		app.add_plugins(VisualGeometryCorePlugin);
	}
	if !app.is_plugin_added::<DurhamTerrainModelsPlugin>() {
		app.add_plugins(DurhamTerrainModelsPlugin);
	}
	if !app.is_plugin_added::<TerrainShaderPlugin>() {
		app.add_plugins(TerrainShaderPlugin);
	}
	install_enforced_mesh_cache::<TerrainMeshBuilder, TerrainShader>(app);
	share_terrain_chunk_refs::<TerrainMeshBuilder>(app, false);
	install_enforced_mesh_cache::<ComposedWater, RefractionWater>(app);

	let layout = TerrainCellLayout::default();
	let config = TerrainConfig::new(0);
	let base = BaseTerrainNoise::from_config(&config);
	app.insert_resource(
		MeshFulfillBudget::<TerrainMeshBuilder>::new(8, 16, 256)
			.with_prefer_xz(layout.region_center_xz()),
	)
	.insert_resource(config)
	.insert_resource(WorldBaseTerrain(base))
	.init_resource::<TerrainCoverage>()
	.insert_resource(layout)
	.insert_resource(TerrainFillParams { coverage: TerrainCoverage::default(), terrain_radius: 1 })
	.init_resource::<TerrainPresentationDirty>()
	.init_resource::<TerrainPresentPending>()
	.init_resource::<TerrainStreaming<Durham>>()
	.init_resource::<TerrainExtent<Durham>>()
	.init_resource::<TerrainLayoutPinned>()
	.insert_resource(LodGenerateBudget::<TerrainWindow>::new(TERRAIN_ADMIT_PER_FRAME))
	.insert_resource(ChannelTimeBudget::<TerrainWindow>::unlimited())
	.add_plugins((
		Seed::<TerrainStampConfigs>::default(),
		Seed::<WatershedConfigs>::default(),
		Seed::<TerrainPresentationAssets>::default(),
		Seed::<WaterPresentationAssets>::default(),
		GenerateOn::<TerrainWindow, Terrain>::in_set(TerrainFillSystems::Subscribe),
		GenerateOn::<TerrainWindow, Water>::in_set(TerrainFillSystems::Subscribe),
	))
	.configure_sets(
		Update,
		(
			TerrainFillSystems::Window
				.in_set(TerrainFillSystems::Generate)
				.after(HcsgSeedSystems)
				.before(TerrainFillSystems::Subscribe),
			TerrainFillSystems::Subscribe.in_set(TerrainFillSystems::Generate),
			TerrainFillSystems::Generate
				.run_if(terrain_streaming::<Durham>)
				.before(TerrainColliderSystems::QueueMeshes)
				.before(TerrainLayerSystems::<Durham>::QueueColliders),
		),
	)
	.add_systems(Startup, setup_presentation_assets)
	.add_systems(Update, write_durham_extent.after(TerrainFillSystems::Generate))
	.add_systems(Update, produce_terrain_window.in_set(TerrainFillSystems::Window))
	.add_systems(
		Update,
		(mark_generated_pending, sync_world_base)
			.in_set(TerrainFillSystems::Generate)
			.after(TerrainFillSystems::Subscribe),
	);
}

/// Apply a mode's seed, coverage, and radius. A seed change rebuilds stores
/// the way a retarget does.
pub(crate) fn apply_durham_generation(world: &mut World, config: &crate::DurhamTerrainConfig) {
	let terrain_radius = config.terrain_radius.max(1);
	let seed_changed = world
		.get_resource::<TerrainConfig>()
		.is_none_or(|live| live.seed != config.seed);
	if seed_changed {
		let terrain = TerrainConfig::new(config.seed);
		let base = BaseTerrainNoise::from_config(&terrain);
		world.insert_resource(WorldBaseTerrain(base));
		if let Some(mut dirty) = world.get_resource_mut::<TerrainPresentationDirty>() {
			dirty.0 = true;
		}
		if let Some(mut pending) = world.get_resource_mut::<TerrainPresentPending>() {
			pending.0 = true;
		}
		if let Some(mut assets) = world.get_resource_mut::<TerrainPresentationAssets>() {
			assets.config = terrain.clone();
		}
		world.insert_resource(terrain);
	}
	world.insert_resource(TerrainFillParams { coverage: config.coverage, terrain_radius });
}

/// Raw Durham present: the three stream presenter states and [`present_cells`].
pub(crate) fn install_durham_presentation(app: &mut App) {
	app.init_resource::<TerrainPresenterState>()
		.init_resource::<LodPresentGate<OnTerrain<Durham>>>()
		.init_resource::<TerrainStreamPresenterState<TerrainNear>>()
		.init_resource::<TerrainStreamPresenterState<TerrainFar>>()
		.init_resource::<TerrainStreamPresenterState<TerrainBackground>>()
		.add_systems(
			Update,
			present_cells
				.after(TerrainFillSystems::Generate)
				.before(TerrainColliderSystems::QueueMeshes)
				.before(TerrainLayerSystems::<Durham>::QueueColliders)
				.run_if(terrain_streaming::<Durham>)
				.run_if(mode_subscribed::<OnTerrain<Durham>>()),
		)
		.add_systems(Update, clear_closed_terrain_present.after(LodPresentGateSync));
}

fn clear_closed_terrain_present(
	gate: Res<LodPresentGate<OnTerrain<Durham>>>,
	mut commands: Commands,
	mut state: ResMut<TerrainPresenterState>,
) {
	if gate.is_changed() && !gate.open {
		state.clear(&mut commands);
	}
}

#[derive(Resource, Clone, Copy)]
struct TerrainFillParams {
	coverage: TerrainCoverage,
	terrain_radius: i32,
}

/// Terrain presentation assets for `coverage`, drawn with `material`.
pub fn presentation_assets(
	config: TerrainConfig,
	material: Handle<TerrainShader>,
	coverage: TerrainCoverage,
	terrain_radius: i32,
) -> TerrainPresentationAssets {
	let mut assets = TerrainPresentationAssets {
		config,
		material,
		lod_bands: Vec::new(),
		outer_add_walls: true,
		fine_grid_max_radius: None,
		macro_seam_half_extents: Vec::new(),
		macro_cell_min_size: None,
		macro_res_2: None,
	};
	retarget_presentation_assets(&mut assets, coverage, terrain_radius);
	assets
}

fn setup_presentation_assets(
	mut commands: Commands,
	mut terrain_materials: ResMut<Assets<TerrainShader>>,
	mut water_materials: ResMut<Assets<RefractionWater>>,
	config: Res<TerrainConfig>,
	params: Res<TerrainFillParams>,
) {
	commands.insert_resource(presentation_assets(
		config.clone(),
		terrain_materials.add(TerrainShader::default()),
		params.coverage,
		params.terrain_radius,
	));
	commands.insert_resource(WaterPresentationAssets {
		material: water_materials.add(RefractionWater::default()),
	});
}

fn viewer_xz(
	lod_viewers: &Query<&GlobalTransform, With<LodViewer>>,
	cameras: &Query<&GlobalTransform, With<Camera3d>>,
) -> Option<Vec3> {
	lod_viewers.iter().next().or_else(|| cameras.iter().next()).map(|tf| {
		let t = tf.translation();
		Vec3::new(t.x, 0.0, t.z)
	})
}

/// Rebuilds after a dirty retarget, recenters the layout on the viewer, seeds
/// it, and republishes its request region on [`TerrainWindow`].
///
/// The layout's rings tile differently around each center, so a moved or
/// retargeted window restarts subscribers rather than scanning entering strips.
#[allow(clippy::too_many_arguments)]
pub fn produce_terrain_window(
	mut storage: ResMut<HcsgStorage>,
	mut layout: ResMut<TerrainCellLayout>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut epoch: ResMut<TerrainColliderEpoch>,
	mut mesh_budget: ResMut<MeshFulfillBudget<TerrainMeshBuilder>>,
	pinned: Res<TerrainLayoutPinned>,
	mut window: GenerationProducer<TerrainWindow>,
	lod_viewers: Query<&GlobalTransform, With<LodViewer>>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
) {
	let mut restart = window.current().is_none();
	if dirty.0 {
		storage.clear_group::<DurhamNodes>();
		epoch.0 = epoch.0.wrapping_add(1);
		dirty.0 = false;
		pending.0 = true;
		restart = true;
	}

	let viewer = viewer_xz(&lod_viewers, &cameras);
	if let Some(xz) = viewer {
		mesh_budget.prefer_xz = Some(xz);
		if !pinned.0 {
			let mut recentered = layout.clone();
			if recentered.recenter_on_xz(xz) {
				*layout = recentered;
				pending.0 = true;
			}
		}
	}

	if storage.get::<TerrainCellLayout>(Id::Universal) != Some(&*layout) {
		storage.seed(layout.clone(), universal_bounds());
		restart = true;
	}

	let priority = viewer.unwrap_or_else(|| layout.region_center_xz()).xz();
	if restart {
		window.restart(layout.request_region(), Some(priority));
	} else {
		window.publish(layout.request_region(), Some(priority));
	}
}

/// Terrain and water finish independently, so either announcement re-presents.
fn mark_generated_pending(
	mut terrain: MessageReader<LodGenerated<Terrain>>,
	mut water: MessageReader<LodGenerated<Water>>,
	mut pending: ResMut<TerrainPresentPending>,
) {
	if terrain.read().count() + water.read().count() > 0 {
		pending.0 = true;
	}
}

/// Camera height reads [`WorldBaseTerrain`]; keep it on the stored base noise.
fn sync_world_base(
	storage: Res<HcsgStorage>,
	mut world_base: ResMut<WorldBaseTerrain>,
	mut synced: Local<Option<Version>>,
) {
	let Some(entry) = storage.entry::<BaseTerrainNoise>(Id::Universal) else {
		return;
	};
	if *synced != Some(entry.version) {
		world_base.0 = entry.value.clone();
		*synced = Some(entry.version);
	}
}

fn present_cells(
	mut terrain_presenter: TerrainRegionPresenter,
	storage: Res<HcsgStorage>,
	layout: Res<TerrainCellLayout>,
	mut pending: ResMut<TerrainPresentPending>,
	lod_viewers: Query<&GlobalTransform, With<LodViewer>>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
) {
	if !pending.0 {
		return;
	}

	let region = layout.presentation_region();
	let viewer = viewer_xz(&lod_viewers, &cameras)
		.map(|xz| Transform::from_translation(xz))
		.unwrap_or(Transform::IDENTITY);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &viewer,
		current_transform: &viewer,
		bounds: &region,
	};
	RegionPresenter::<Terrain, _>::present(&mut terrain_presenter, &*storage, region, &lod_ref);
	terrain_presenter.sync_water();
	let wanted: HashSet<Id> = storage.terrain_ids_overlapping(region).into_iter().collect();
	terrain_presenter.remove_stale(&wanted);
	pending.0 = false;
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::terrain::cell::CellTiling;
	use crate::terrain::index::register_durham_nodes;
	use crate::DurhamTerrainConfig;

	#[test]
	fn world_near_high_stays_at_eight_cells() {
		assert_eq!(WORLD_FINE_HALF_EXTENT_CELLS, 8);
		assert_eq!(world_lod_bands(), vec![TerrainMeshLodBand { max_radius_cells: 8, res_2: 5 }]);
	}

	#[test]
	fn world_stream_rings_are_three_high_only_scales() {
		let layout = world_cell_layout();
		assert_eq!(layout.stream_rings.len(), 3);
		assert_eq!(layout.stream_rings[0].cell_size, TERRAIN_CELL_SIZE);
		assert_eq!(layout.stream_rings[1].cell_size, 2.0 * TERRAIN_CELL_SIZE);
		assert_eq!(layout.stream_rings[2].cell_size, 4.0 * TERRAIN_CELL_SIZE);
		assert_eq!(layout.stream_rings[0].res_2, 5);
		assert_eq!(layout.stream_rings[1].res_2, 4);
		assert_eq!(layout.stream_rings[2].res_2, 3);
		assert!(layout.stream_rings[0].seeds_collision());
		assert!(!layout.stream_rings[1].seeds_collision());
		assert!(!layout.stream_rings[2].seeds_collision());
		let ids = layout.cell_ids(layout.request_region());
		assert!(!ids.is_empty());
	}

	#[test]
	fn world_far_interior_is_empty_high() {
		let layout = world_cell_layout();
		let far = layout.stream_rings[1];
		assert!(!far.draws_high());
		assert!(far.draws_level(lod::LodSceneLevel::Medium));
		assert_eq!(far.level_for(Vec3::ZERO, Vec3::ZERO), lod::LodSceneLevel::High);
		assert_eq!(
			far.level_for(Vec3::X * 5.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			lod::LodSceneLevel::High
		);
		assert_eq!(
			far.level_for(Vec3::X * 7.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			lod::LodSceneLevel::Medium
		);
		assert_eq!(
			far.level_for(Vec3::X * 12.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			lod::LodSceneLevel::Medium
		);
		let background = layout.stream_rings[2];
		assert_eq!(background.level_for(Vec3::ZERO, Vec3::ZERO), lod::LodSceneLevel::High);
		assert_eq!(
			background.level_for(Vec3::X * 13.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			lod::LodSceneLevel::Medium
		);
		assert!(background.draws_level(lod::LodSceneLevel::Medium));
	}

	#[test]
	fn world_stream_boundaries_align_all_three_grids() {
		assert_eq!(WORLD_TERRAIN_NEAR_RADIUS_M % (2.0 * TERRAIN_CELL_SIZE), 0.0);
		assert_eq!(WORLD_TERRAIN_FAR_RADIUS_M % (4.0 * TERRAIN_CELL_SIZE), 0.0);
		assert_eq!(WORLD_TERRAIN_BACKGROUND_RADIUS_M % (4.0 * TERRAIN_CELL_SIZE), 0.0);
	}

	#[test]
	fn world_layout_recenters_with_the_viewer() {
		let mut layout = world_cell_layout();
		let size = TERRAIN_CELL_SIZE;
		assert!(layout.recenter_on_xz(Vec3::X * 20.0 * size));
		assert_eq!(layout.origin, IVec2::new(12, -WORLD_FINE_HALF_EXTENT_CELLS));
		let ids = layout.cell_ids(layout.request_region());
		assert!(!ids.is_empty());
	}

	#[test]
	fn world_stream_keep_stays_inside_seven_km() {
		let outer = WORLD_TERRAIN_BACKGROUND_RADIUS_M + WORLD_TERRAIN_CULL_MARGIN_M;
		assert!(outer < 7_000.0, "playable keep half-extent {outer}");
	}

	#[test]
	fn playable_generate_keep_matches_drawn_bands() {
		let layout = world_cell_layout();
		for ring in &layout.stream_rings {
			assert_eq!(ring.cull_margin, 0.0);
		}
		let far = layout.stream_rings[1];
		assert_eq!(
			far.high_inner_radius,
			WORLD_TERRAIN_NEAR_RADIUS_M - WORLD_TERRAIN_FAR_HOLE_INSET_M
		);
		assert!(!far.retains_cell_center(Vec3::ZERO, Vec3::ZERO));
		assert!(!far.retains_cell_center(Vec3::X * 5.0 * TERRAIN_CELL_SIZE, Vec3::ZERO));
		assert!(far.retains_cell_center(Vec3::X * 7.0 * TERRAIN_CELL_SIZE, Vec3::ZERO));
		assert!(far.retains_cell_center(Vec3::X * 12.0 * TERRAIN_CELL_SIZE, Vec3::ZERO));
		let background = layout.stream_rings[2];
		assert_eq!(
			background.high_inner_radius,
			WORLD_TERRAIN_FAR_RADIUS_M - WORLD_TERRAIN_BACKGROUND_HOLE_INSET_M
		);
		let near = layout.stream_rings[0];
		assert!(near.retains_cell_center(Vec3::ZERO, Vec3::ZERO));
		assert!(!near.retains_cell_center(
			Vec3::X * (WORLD_TERRAIN_NEAR_RADIUS_M + TERRAIN_CELL_SIZE),
			Vec3::ZERO
		));
	}

	#[test]
	fn only_background_stream_emits_outer_skirts() {
		let layout = world_cell_layout();
		assert!(!layout.is_outermost_stream_ring(layout.stream_rings[0]));
		assert!(!layout.is_outermost_stream_ring(layout.stream_rings[1]));
		assert!(layout.is_outermost_stream_ring(layout.stream_rings[2]));
	}

	#[test]
	fn fine_patch_is_a_four_cell_grid() {
		let layout = fine_patch_cell_layout(2, IVec2::new(-2, -2));
		assert!(!layout.is_streamed());
		assert_eq!(layout.extents, UVec2::new(4, 4));
		assert_eq!(layout.origin, IVec2::new(-2, -2));
		assert!(!TerrainLayoutPinned::default().0);
	}

	#[test]
	fn fine_patch_sits_on_its_origin() {
		let origin = IVec2::new(5, -5);
		let layout = fine_patch_cell_layout(2, origin);
		assert_eq!(layout.origin, origin);
		assert_eq!(layout.extents, UVec2::new(4, 4));
		let center = layout.region_center_xz();
		assert!((center.x - (origin.x + 2) as f32 * layout.cell_size).abs() < 1e-3);
		assert!((center.z - (origin.y + 2) as f32 * layout.cell_size).abs() < 1e-3);
	}

	#[test]
	fn retarget_clears_playable_macro_bands_on_a_fine_patch() {
		let mut assets = TerrainPresentationAssets {
			config: TerrainConfig::new(42),
			material: Handle::default(),
			lod_bands: world_lod_bands(),
			outer_add_walls: true,
			fine_grid_max_radius: Some(WORLD_FINE_HALF_EXTENT_CELLS),
			macro_seam_half_extents: vec![WORLD_TERRAIN_NEAR_RADIUS_M],
			macro_cell_min_size: Some(2.0 * TERRAIN_CELL_SIZE),
			macro_res_2: Some(3),
		};
		retarget_presentation_assets(&mut assets, TerrainCoverage::FinePatch, 2);
		assert_eq!(assets.lod_bands, playground_lod_bands(2));
		assert_eq!(assets.fine_grid_max_radius, Some(2));
		assert!(assets.macro_seam_half_extents.is_empty());
		assert!(assets.macro_cell_min_size.is_none());
		assert!(assets.macro_res_2.is_none());
	}

	#[test]
	fn seed_change_keeps_store_until_dirty_generation_clears_it() -> anyhow::Result<()> {
		let mut world = World::new();
		let layout = TerrainCellLayout::default();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(1));
		let mut storage = HcsgStorage::default();
		register_durham_nodes(&mut storage);
		world.insert_resource(storage);
		world.insert_resource(TerrainConfig::new(1));
		world.insert_resource(TerrainPresentationDirty(false));
		world.insert_resource(TerrainPresentPending(false));
		world.insert_resource(TerrainPresenterState::default());
		let present_entity = world.spawn_empty().id();
		let (id, presented, revision) = {
			let mut storage = world.resource_mut::<HcsgStorage>();
			storage.insert_base_terrain_for_test(&layout, 0, 0, base);
			let id = storage.terrain_ids_overlapping(layout.request_region())[0];
			let presented =
				storage.entry::<Terrain>(id).ok_or_else(|| anyhow::anyhow!("inserted"))?.version;
			(id, presented, storage.latest_version())
		};
		world.resource_mut::<TerrainPresenterState>().insert_for_test(
			id,
			presented,
			present_entity,
		);

		apply_durham_generation(
			&mut world,
			&DurhamTerrainConfig {
				seed: 99,
				coverage: TerrainCoverage::FinePatch,
				terrain_radius: 1,
			},
		);

		assert!(world.get_entity(present_entity).is_ok(), "the presenter survives apply");
		{
			let storage = world.resource::<HcsgStorage>();
			assert_eq!(
				storage.entry::<Terrain>(id).map(|entry| entry.version),
				Some(presented),
				"apply must not rewind or replace the store"
			);
			assert_eq!(storage.latest_version(), revision);
		}
		assert_eq!(
			world.resource::<TerrainPresenterState>().presented_version(id),
			Some(presented)
		);
		assert!(world.resource::<TerrainPresentationDirty>().0);

		let mut storage = world.resource_mut::<HcsgStorage>();
		storage.clear_group::<DurhamNodes>();
		let rebuilt_base = BaseTerrainNoise::from_config(&TerrainConfig::new(99));
		storage.insert_base_terrain_for_test(&layout, 0, 0, rebuilt_base);
		let rebuilt = storage
			.entry::<Terrain>(id)
			.ok_or_else(|| anyhow::anyhow!("reinserted"))?
			.version;
		assert!(
			rebuilt > presented,
			"dirty clear must mint a version the surviving presenter will accept"
		);
		Ok(())
	}

	fn empty_water(cell: bevy::math::bounding::Aabb3d) -> Water {
		let terrain = crate::terrain::sdf::TerrainSdf::new(1, 20.0);
		Water {
			cell,
			sdf: ComposedWater::compose(terrain.clone(), Vec::new()),
			terrain,
			fills: Vec::new(),
			material: Handle::default(),
			res_2: 4,
			stream_ring: None,
		}
	}

	#[test]
	fn water_generated_after_its_terrain_still_attaches() -> anyhow::Result<()> {
		let mut app = App::new();
		let layout = TerrainCellLayout::default();
		let mut storage = HcsgStorage::default();
		register_durham_nodes(&mut storage);
		app.insert_resource(storage)
			.insert_resource(layout.clone())
			.insert_resource(TerrainPresentPending(false))
			.init_resource::<TerrainPresenterState>()
			.add_message::<LodGenerated<Terrain>>()
			.add_message::<LodGenerated<Water>>()
			.add_systems(Update, (mark_generated_pending, present_cells).chain());

		let id = {
			let mut storage = app.world_mut().resource_mut::<HcsgStorage>();
			let base = BaseTerrainNoise::from_config(&TerrainConfig::new(1));
			storage.insert_base_terrain_for_test(&layout, 0, 0, base);
			storage.terrain_ids_overlapping(layout.request_region())[0]
		};
		app.world_mut().write_message(LodGenerated::<Terrain>::new(id));
		app.update();

		let terrain_version = {
			let state = app.world().resource::<TerrainPresenterState>();
			assert!(state.presented_water(id).is_none(), "no water generated yet");
			state
				.presented_version(id)
				.ok_or_else(|| anyhow::anyhow!("terrain presented"))?
		};

		let cell = app
			.world()
			.resource::<HcsgStorage>()
			.entry::<Terrain>(id)
			.ok_or_else(|| anyhow::anyhow!("terrain stored"))?
			.value
			.cell;
		let first =
			app.world_mut()
				.resource_mut::<HcsgStorage>()
				.insert(id, empty_water(cell), cell);
		app.world_mut().write_message(LodGenerated::<Water>::new(id));
		app.update();

		let (first_entity, attached) = app
			.world()
			.resource::<TerrainPresenterState>()
			.presented_water(id)
			.ok_or_else(|| anyhow::anyhow!("late water must attach to presented terrain"))?;
		assert_eq!(attached, first);
		assert_eq!(
			app.world().resource::<TerrainPresenterState>().presented_version(id),
			Some(terrain_version),
			"water arrival must not respawn the terrain"
		);

		let second =
			app.world_mut()
				.resource_mut::<HcsgStorage>()
				.insert(id, empty_water(cell), cell);
		app.world_mut().write_message(LodGenerated::<Water>::new(id));
		app.update();

		let (second_entity, attached) = app
			.world()
			.resource::<TerrainPresenterState>()
			.presented_water(id)
			.ok_or_else(|| anyhow::anyhow!("rebuilt water stays attached"))?;
		assert_eq!(attached, second);
		assert_ne!(second_entity, first_entity);
		assert!(app.world().get_entity(first_entity).is_err(), "stale water despawns");
		Ok(())
	}
}
