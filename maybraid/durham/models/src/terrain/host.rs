//! World-facing streamed terrain: models, shaders, mesh caches, and fill present.
//!
//! Playable coverage keeps the 16-cell fine disk plus the existing 2×/4× macro
//! rings. This plugin does not change semantic generation; it hosts the same
//! patch `get_or_generate_region` fill the vegetation crate used to own.

use std::marker::PhantomData;

use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use durham_terrain::shaders::{DurhamTerrainShader, DurhamTerrainShaderPlugin, RefractionWater};
use lod::gen::GeneratingSpatialIndex;
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use visual_geometry_core::{
	install_enforced_mesh_cache, share_terrain_chunk_refs, VisualGeometryCorePlugin,
};

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::cell::{OuterCellRing, TerrainCellLayout, TERRAIN_CELL_SIZE};
use crate::terrain::config::TerrainConfig;
use crate::terrain::index::AvianTerrainIndex;
use crate::terrain::presentation::{
	TerrainMeshLodBand, TerrainPresentationAssets, TerrainRegionPresenter, TerrainStoreView,
};
use crate::water::{
	ComposedWater, Water, WaterPresentationAssets, WaterRegionPresenter, WaterStoreView,
};
use crate::{DurhamTerrainModelsPlugin, Terrain, TerrainMeshBuilder};

/// Composed Durham SDF / CpuShot terrain model.
pub struct Durham;

/// Fine-grid Chebyshev half-extent (16 × 160 m ≈ 2.6 km).
pub const WORLD_FINE_HALF_EXTENT_CELLS: i32 = 16;
/// 2× macro ring past the fine grid.
pub const WORLD_OUTER_2X_ROWS: i32 = 2;
/// 4× macro ring past the 2× ring.
pub const WORLD_OUTER_4X_ROWS: i32 = 1;

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

/// When true, fill should `get_or_generate_region` for the current layout.
#[derive(Resource)]
pub struct TerrainPresentationDirty(pub bool);

/// When true, fill meshes have generated and present is still owed.
#[derive(Resource, Default)]
pub struct TerrainPresentPending(pub bool);

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
		install_enforced_mesh_cache::<TerrainMeshBuilder, DurhamTerrainShader>(app);
		share_terrain_chunk_refs::<TerrainMeshBuilder>(app, false);
		install_enforced_mesh_cache::<ComposedWater, RefractionWater>(app);

		app.insert_resource(config)
			.insert_resource(WorldBaseTerrain(base))
			.insert_resource(coverage)
			.insert_resource(layout_for(coverage, terrain_radius))
			.insert_resource(TerrainFillParams { coverage, terrain_radius })
			.insert_resource(TerrainPresentationDirty(true))
			.init_resource::<TerrainPresentPending>()
			.add_systems(Startup, setup_presentation_assets)
			.add_systems(Update, (generate_cells, present_cells.after(generate_cells)));
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

fn generate_cells(
	mut index: AvianTerrainIndex,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut world_base: ResMut<WorldBaseTerrain>,
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

	dirty.0 = false;
	pending.0 = true;
}

fn present_cells(
	mut terrain_presenter: TerrainRegionPresenter,
	mut water_presenter: WaterRegionPresenter,
	store: Res<crate::terrain::index::TerrainEntryStore>,
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::terrain::cell::origin_cell_ids_for_layout;

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
