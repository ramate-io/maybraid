//! World-facing streamed terrain: models, shaders, mesh caches, and LOD.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin, RefractionWater};
use lod::gen::LodGenerateKeepRegion;
use lod::lod_ref::LodRef;
use lod::presentation::LodPresentKeepRegion;
use lod::{
	LodCullRegionCursor, LodCullRegions, LodCullRegionsStatus, LodGenerateSystems,
	LodPresentSystems, LodRefreshRegions, LodRefreshRegionsStatus, LodViewer, OpenLattice,
};
use visual_geometry_core::{
	install_enforced_mesh_cache, share_terrain_chunk_refs, VisualGeometryCorePlugin,
};

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::cell::{TerrainCellLayout, TerrainCellRing, TERRAIN_CELL_SIZE};
use crate::terrain::config::TerrainConfig;
use crate::terrain::presentation::{TerrainMeshLodBand, TerrainPresentationAssets};
use crate::terrain::stream::TerrainLodPlugin;
use crate::water::{ComposedWater, WaterPresentationAssets};
use crate::DurhamTerrainModelsPlugin;

/// Composed Durham SDF / CpuShot terrain model.
pub struct Durham;

/// Fine-only patch vs playable world extents (fine grid + macro rings).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerrainCoverage {
	#[default]
	FinePatch,
	PlayableWorld,
}

/// Near-stream High half-extent (8 × 160 m = 1.28 km).
pub const WORLD_FINE_HALF_EXTENT_CELLS: i32 = 8;
/// Near 160 m cells end on a boundary shared by the 320 m grid.
pub const WORLD_TERRAIN_NEAR_RADIUS_M: f32 = 8.0 * TERRAIN_CELL_SIZE;
/// Far 320 m cells end on a boundary shared by the 640 m grid.
pub const WORLD_TERRAIN_FAR_RADIUS_M: f32 = 16.0 * TERRAIN_CELL_SIZE;
/// Background 640 m cells provide the horizon out to the existing world edge.
pub const WORLD_TERRAIN_BACKGROUND_RADIUS_M: f32 = 24.0 * TERRAIN_CELL_SIZE;
/// Keep one near-band width of empty hosts around each visible band.
pub const WORLD_TERRAIN_CULL_MARGIN_M: f32 = WORLD_TERRAIN_NEAR_RADIUS_M;
pub const WORLD_TERRAIN_STREAM_EDGE_M: f32 =
	2.0 * (WORLD_TERRAIN_BACKGROUND_RADIUS_M + WORLD_TERRAIN_CULL_MARGIN_M);
/// Keep annulus boundaries aligned to the coarsest (640 m) global grid.
pub const WORLD_TERRAIN_PRESENT_STEP_M: f32 = 4.0 * TERRAIN_CELL_SIZE;
const WORLD_TERRAIN_QUERY_MIN_Y: f32 = -8_000.0;
const WORLD_TERRAIN_QUERY_MAX_Y: f32 = 8_000.0;

/// Base noise used for camera height before (and alongside) generation.
#[derive(Resource)]
pub struct WorldBaseTerrain(pub BaseTerrainNoise);

/// Whether terrain and its dependent vegetation streams may advance.
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

/// Streamed terrain stack for model `M` (currently [`Durham`]).
pub struct TerrainPlugin<M> {
	_marker: PhantomData<fn() -> M>,
	pub seed: u32,
	pub coverage: TerrainCoverage,
	pub terrain_radius: i32,
}

impl TerrainPlugin<Durham> {
	pub fn fine_patch(terrain_radius: i32) -> Self {
		Self {
			_marker: PhantomData,
			seed: 42,
			coverage: TerrainCoverage::FinePatch,
			terrain_radius: terrain_radius.max(1),
		}
	}

	pub fn playable_world() -> Self {
		Self {
			_marker: PhantomData,
			seed: 42,
			coverage: TerrainCoverage::PlayableWorld,
			terrain_radius: WORLD_FINE_HALF_EXTENT_CELLS,
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
		install_enforced_mesh_cache::<crate::TerrainMeshBuilder, DurhamTerrainShader>(app);
		share_terrain_chunk_refs::<crate::TerrainMeshBuilder>(app, false);
		install_enforced_mesh_cache::<ComposedWater, RefractionWater>(app);

		app.add_plugins(
			TerrainLodPlugin::<TerrainLodRegion, With<LodViewer>, TerrainLodChannel>::default(),
		)
		.insert_resource(config)
		.insert_resource(WorldBaseTerrain(base))
		.insert_resource(coverage)
		.init_resource::<TerrainStreamingEnabled>()
		.insert_resource(layout_for(coverage, terrain_radius))
		.insert_resource(TerrainPresentationDirty(true))
		.insert_resource(TerrainLodRegion {
			moving: coverage == TerrainCoverage::PlayableWorld,
			..default()
		})
		.add_systems(Startup, setup_presentation_assets)
		.add_systems(
			Update,
			initialize_terrain_lod_regions
				.before(LodGenerateSystems::Produce)
				.before(LodPresentSystems::Produce)
				.run_if(terrain_streaming_enabled),
		);
	}
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

#[derive(Resource)]
pub struct TerrainPresentationDirty(pub bool);

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

	fn lod_current_region(&self, lod_ref: &LodRef) -> Option<Aabb3d> {
		self.moving.then(|| {
			terrain_ring_region(
				lod_ref.current_transform.translation,
				WORLD_TERRAIN_STREAM_EDGE_M,
				WORLD_TERRAIN_PRESENT_STEP_M,
			)
		})
	}
}

fn setup_presentation_assets(
	mut commands: Commands,
	mut terrain_materials: ResMut<Assets<DurhamTerrainShader>>,
	mut water_materials: ResMut<Assets<RefractionWater>>,
	config: Res<TerrainConfig>,
	coverage: Res<TerrainCoverage>,
	layout: Res<TerrainCellLayout>,
) {
	let material = terrain_materials.add(DurhamTerrainShader::default());
	let (macro_seam_half_extents, macro_cell_min_size, macro_res_2) = match *coverage {
		TerrainCoverage::FinePatch => (Vec::new(), None, None),
		TerrainCoverage::PlayableWorld => (Vec::new(), Some(2.0 * TERRAIN_CELL_SIZE), Some(5)),
	};
	let terrain_radius = (layout.extents.x as i32 / 2).max(1);
	commands.insert_resource(TerrainPresentationAssets {
		config: config.clone(),
		material,
		lod_bands: lod_bands_for(*coverage, terrain_radius),
		outer_add_walls: true,
		fine_grid_max_radius: (*coverage == TerrainCoverage::FinePatch).then_some(terrain_radius),
		macro_seam_half_extents,
		macro_cell_min_size,
		macro_res_2,
	});
	commands.insert_resource(WaterPresentationAssets {
		material: water_materials.add(RefractionWater::default()),
	});
}

fn initialize_terrain_lod_regions(
	coverage: Res<TerrainCoverage>,
	layout: Res<TerrainCellLayout>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	viewers: Query<&Transform, With<LodViewer>>,
	mut generate_keep: ResMut<LodGenerateKeepRegion<TerrainLodChannel>>,
	mut present_keep: ResMut<LodPresentKeepRegion<TerrainLodChannel>>,
) {
	if !dirty.0 && generate_keep.region.is_some() && present_keep.region.is_some() {
		return;
	}
	let region = match *coverage {
		TerrainCoverage::FinePatch => layout.request_region(),
		TerrainCoverage::PlayableWorld => {
			let position =
				viewers.single().map(|transform| transform.translation).unwrap_or(Vec3::ZERO);
			terrain_ring_region(position, WORLD_TERRAIN_STREAM_EDGE_M, WORLD_TERRAIN_PRESENT_STEP_M)
		}
	};
	generate_keep.region = Some(region);
	present_keep.region = Some(region);
	dirty.0 = false;
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::terrain::cell::origin_cell_ids_for_layout;

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
