//! World-facing streamed terrain: models, shaders, mesh caches, and fill present.
//!
//! Playable coverage is three scale streams (160 / 320 / 640 m) that follow the
//! viewer. Near cells use `res_2 = 5` and own collision; far and background are
//! render-only at `res_2 = 4` and `3`. Far / Background holes inset so Medium
//! overlaps the next-finer High rim. Generation admits a bounded number of
//! missing origin ids per frame. Playable visuals come from the urbanized
//! presenter. This plugin generates Durham on every coverage; raw present is
//! FinePatch-only (`present: true`).

use std::marker::PhantomData;

use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin, RefractionWater};
use lod::gen::{GeneratingSpatialIndex, Id, OriginalId, SpatialIndex, StorageStatus};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use lod::LodViewer;
use render_item::mesh::handle::MeshFulfillBudget;
use std::collections::HashSet;
use visual_geometry_core::{
	install_enforced_mesh_cache, share_terrain_chunk_refs, VisualGeometryCorePlugin,
};

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::cell::{
	origin_cell_ids_for_layout, TerrainCellLayout, TerrainCellRing, TERRAIN_CELL_SIZE,
};
use crate::terrain::collider::{TerrainColliderEpoch, TerrainColliderSystems};
use crate::terrain::config::TerrainConfig;
use crate::terrain::index::AvianTerrainIndex;
use crate::terrain::presentation::{
	TerrainBackground, TerrainFar, TerrainMeshLodBand, TerrainNear, TerrainPresentationAssets,
	TerrainRegionPresenter, TerrainStoreView, TerrainStreamPresenterState,
};
use crate::water::{ComposedWater, Water, WaterPresentationAssets};
use crate::{DurhamTerrainModelsPlugin, Terrain, TerrainMeshBuilder};

/// Composed Durham SDF / CpuShot terrain model.
pub struct Durham;

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
const TERRAIN_ADMIT_PER_FRAME: usize = 4;

/// Fine-only patch vs playable world extents (fine grid + macro rings).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerrainCoverage {
	#[default]
	FinePatch,
	PlayableWorld,
}

/// Base noise used for camera height before (and alongside) generation.
#[derive(Resource)]
pub struct WorldBaseTerrain(pub BaseTerrainNoise);

/// When true, fill should clear and rebuild (playground radius / seed commands).
#[derive(Resource)]
pub struct TerrainPresentationDirty(pub bool);

/// When false, Durham generate runs but this plugin does not present raw
/// terrain or seed raw [`crate::terrain::Terrain::scene`] colliders.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainPresentEnabled(pub bool);

impl Default for TerrainPresentEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// Whether terrain fill and dependent vegetation streams may advance.
///
/// Playgrounds default this on. The game shell keeps it off on Home / Characters
/// so the menu does not eagerly build the world.
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
			high_inner_radius: WORLD_TERRAIN_NEAR_RADIUS_M - WORLD_TERRAIN_FAR_HOLE_INSET_M,
			high_outer_radius: WORLD_TERRAIN_FAR_RADIUS_M,
			cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
		},
		TerrainCellRing {
			cell_size: 4.0 * TERRAIN_CELL_SIZE,
			res_2: 3,
			anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
			high_inner_radius: WORLD_TERRAIN_FAR_RADIUS_M - WORLD_TERRAIN_BACKGROUND_HOLE_INSET_M,
			high_outer_radius: WORLD_TERRAIN_BACKGROUND_RADIUS_M,
			cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
		},
	];
	layout
}

fn layout_for(coverage: TerrainCoverage, terrain_radius: i32) -> TerrainCellLayout {
	match coverage {
		TerrainCoverage::FinePatch => cell_layout(terrain_radius),
		TerrainCoverage::PlayableWorld => world_cell_layout(),
	}
}

fn lod_bands_for(coverage: TerrainCoverage, terrain_radius: i32) -> Vec<TerrainMeshLodBand> {
	match coverage {
		TerrainCoverage::FinePatch => playground_lod_bands(terrain_radius),
		TerrainCoverage::PlayableWorld => world_lod_bands(),
	}
}

/// Streamed terrain stack for model `M` (currently [`Durham`]).
pub struct TerrainPlugin<M> {
	_marker: PhantomData<fn() -> M>,
	pub seed: u32,
	pub coverage: TerrainCoverage,
	pub terrain_radius: i32,
	/// Raw Durham present + raw collider seed. Playable world leaves this off
	/// so urbanized terrain is the only presented model.
	pub present: bool,
}

impl TerrainPlugin<Durham> {
	pub fn fine_patch(terrain_radius: i32) -> Self {
		Self {
			_marker: PhantomData,
			seed: 42,
			coverage: TerrainCoverage::FinePatch,
			terrain_radius: terrain_radius.max(1),
			present: true,
		}
	}

	pub fn playable_world() -> Self {
		Self {
			_marker: PhantomData,
			seed: 42,
			coverage: TerrainCoverage::PlayableWorld,
			terrain_radius: WORLD_FINE_HALF_EXTENT_CELLS,
			present: false,
		}
	}
}

impl Default for TerrainPlugin<Durham> {
	fn default() -> Self {
		Self::fine_patch(2)
	}
}

impl Plugin for TerrainPlugin<Durham> {
	fn build(&self, app: &mut App) {
		let config = TerrainConfig::new(self.seed);
		let base = BaseTerrainNoise::from_config(&config);
		let coverage = self.coverage;
		let terrain_radius = self.terrain_radius.max(1);

		if !app.is_plugin_added::<VisualGeometryCorePlugin>() {
			app.add_plugins(VisualGeometryCorePlugin);
		}
		if !app.is_plugin_added::<DurhamTerrainModelsPlugin>() {
			app.add_plugins(DurhamTerrainModelsPlugin);
		}
		if !app.is_plugin_added::<DurhamTerrainShaderPlugin>() {
			app.add_plugins(DurhamTerrainShaderPlugin);
		}
		install_enforced_mesh_cache::<TerrainMeshBuilder, DurhamTerrainShader>(app);
		share_terrain_chunk_refs::<TerrainMeshBuilder>(app, false);
		install_enforced_mesh_cache::<ComposedWater, RefractionWater>(app);

		let layout = layout_for(coverage, terrain_radius);
		app.insert_resource(
			MeshFulfillBudget::<TerrainMeshBuilder>::new(8, 16, 256)
				.with_prefer_xz(layout.region_center_xz()),
		)
		.insert_resource(config)
		.insert_resource(WorldBaseTerrain(base))
		.insert_resource(coverage)
		.insert_resource(layout)
		.insert_resource(TerrainFillParams { coverage, terrain_radius })
		.insert_resource(TerrainPresentationDirty(true))
		.insert_resource(TerrainPresentEnabled(self.present))
		.init_resource::<TerrainPresentPending>()
		.init_resource::<TerrainStreamingEnabled>()
		.init_resource::<TerrainStreamPresenterState<TerrainNear>>()
		.init_resource::<TerrainStreamPresenterState<TerrainFar>>()
		.init_resource::<TerrainStreamPresenterState<TerrainBackground>>()
		.add_systems(Startup, setup_presentation_assets)
		.add_systems(
			Update,
			generate_cells
				.run_if(terrain_streaming_enabled)
				.before(TerrainColliderSystems::QueueMeshes),
		);
		if self.present {
			app.add_systems(
				Update,
				present_cells
					.after(generate_cells)
					.before(TerrainColliderSystems::QueueMeshes)
					.run_if(terrain_streaming_enabled),
			);
		}
	}
}

#[derive(Resource, Clone, Copy)]
struct TerrainFillParams {
	coverage: TerrainCoverage,
	terrain_radius: i32,
}

fn setup_presentation_assets(
	mut commands: Commands,
	mut terrain_materials: ResMut<Assets<DurhamTerrainShader>>,
	mut water_materials: ResMut<Assets<RefractionWater>>,
	config: Res<TerrainConfig>,
	params: Res<TerrainFillParams>,
) {
	let material = terrain_materials.add(DurhamTerrainShader::default());
	let (macro_seam_half_extents, macro_cell_min_size, macro_res_2) = match params.coverage {
		TerrainCoverage::FinePatch => (Vec::new(), None, None),
		TerrainCoverage::PlayableWorld => (
			vec![WORLD_TERRAIN_NEAR_RADIUS_M, WORLD_TERRAIN_FAR_RADIUS_M],
			Some(2.0 * TERRAIN_CELL_SIZE),
			Some(3),
		),
	};
	commands.insert_resource(TerrainPresentationAssets {
		config: config.clone(),
		material,
		lod_bands: lod_bands_for(params.coverage, params.terrain_radius),
		outer_add_walls: true,
		fine_grid_max_radius: Some(params.terrain_radius),
		macro_seam_half_extents,
		macro_cell_min_size,
		macro_res_2,
	});
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

fn origin_xz_distance_sq(id: Id, viewer: Vec3) -> f32 {
	let Some(bounds) = id.origin_cell_bounds() else {
		return f32::MAX;
	};
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	let dx = (min.x + max.x) * 0.5 - viewer.x;
	let dz = (min.z + max.z) * 0.5 - viewer.z;
	dx * dx + dz * dz
}

fn generate_cells(
	mut index: AvianTerrainIndex,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut world_base: ResMut<WorldBaseTerrain>,
	mut epoch: ResMut<TerrainColliderEpoch>,
	mut mesh_budget: ResMut<MeshFulfillBudget<TerrainMeshBuilder>>,
	mut window_filled: Local<bool>,
	lod_viewers: Query<&GlobalTransform, With<LodViewer>>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
) {
	if dirty.0 {
		index.clear();
		epoch.0 = epoch.0.wrapping_add(1);
		dirty.0 = false;
		pending.0 = true;
		*window_filled = false;
	}

	let viewer = viewer_xz(&lod_viewers, &cameras);
	if let Some(xz) = viewer {
		mesh_budget.prefer_xz = Some(xz);
		let mut layout = index.layout().clone();
		if layout.recenter_on_xz(xz) {
			index.set_layout(layout);
			pending.0 = true;
			*window_filled = false;
		}
	}

	let layout = index.layout().clone();
	let region = layout.request_region();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	index.publish_layout_if_changed(&lod_ref);

	if *window_filled {
		if let Some(base) = index.base_noise() {
			world_base.0 = base.clone();
		}
		return;
	}

	let prefer = viewer.unwrap_or_else(|| layout.region_center_xz());
	let mut missing: Vec<Id> = origin_cell_ids_for_layout(&layout, region)
		.into_iter()
		.map(|OriginalId(id)| id)
		.filter(|id| {
			<AvianTerrainIndex as SpatialIndex<Terrain>>::storage_status(&index, *id)
				== StorageStatus::NotTracked
		})
		.collect();
	if missing.is_empty() {
		*window_filled = true;
		if let Some(base) = index.base_noise() {
			world_base.0 = base.clone();
		}
		return;
	}
	missing.sort_by(|a, b| {
		origin_xz_distance_sq(*a, prefer)
			.partial_cmp(&origin_xz_distance_sq(*b, prefer))
			.unwrap_or(std::cmp::Ordering::Equal)
	});

	let _span = bevy::log::debug_span!("durham_terrain_generate").entered();
	let mut created = 0usize;
	for id in missing.into_iter().take(TERRAIN_ADMIT_PER_FRAME) {
		if GeneratingSpatialIndex::<Terrain>::get_or_generate(&mut index, id, &lod_ref).is_some() {
			created += 1;
		}
		let _ = GeneratingSpatialIndex::<Water>::get_or_generate(&mut index, id, &lod_ref);
	}
	if created > 0 {
		debug!("admitted terrain_cells={created}");
		pending.0 = true;
	}

	if let Some(base) = index.base_noise() {
		world_base.0 = base.clone();
	}
}

fn present_cells(
	mut terrain_presenter: TerrainRegionPresenter,
	store: Res<crate::terrain::index::TerrainEntryStore>,
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
	let terrain_view = TerrainStoreView::new(&store, &layout);
	RegionPresenter::<Terrain, _>::present(&mut terrain_presenter, &terrain_view, region, &lod_ref);
	let wanted: HashSet<Id> = terrain_view
		.tracked_ids_for(region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	terrain_presenter.remove_stale(&wanted);
	pending.0 = false;
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::terrain::cell::origin_cell_ids_for_layout;

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
		let ids = origin_cell_ids_for_layout(&layout, layout.request_region());
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
		let ids = origin_cell_ids_for_layout(&layout, layout.request_region());
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
	fn playable_world_disables_raw_presentation() {
		assert!(!TerrainPlugin::<Durham>::playable_world().present);
		assert!(TerrainPlugin::<Durham>::fine_patch(2).present);
	}
}
