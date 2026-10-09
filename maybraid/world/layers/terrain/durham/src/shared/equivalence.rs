//! Native schemes against their legacy twins: the same roots generate the
//! same nodes, terrain, and water through [`lod::hcsg::HcsgStorage`] and
//! [`GenerationContext`].

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{Id, OriginalId};
use lod::hcsg::shared::{self, GenerationContext};
use lod::hcsg::HcsgStorage;

use crate::terrain::cell::{universal_bounds, OuterCellRing, TERRAIN_CELL_SIZE};
use crate::terrain::index::{register_durham_nodes, DurhamNodes};
use crate::terrain::stamps::{
	CanyonLowPassStampCell, MassifHighPassControllerCell, PlateauHighPassStampCell,
	PocketWaterLowPassStampCell, RollingLowPassStampCell, ValleyHighPassStampCell,
};
use crate::terrain::watersheds::{
	HydroComplexCell, PocketHighPassCell, PocketWatersHighPass, PocketWatersLowPass,
	PrePocketLowPassCell, WatershedAproningCell, WatershedCarvingCell, WatershedRimmingCell,
};
use crate::terrain::{
	fine_patch_cell_layout, PreWatershedTerrain, Terrain, TerrainCellLayout, TerrainConfig,
	TerrainMeshLodBand, TerrainPresentationAssets, TerrainStampConfigs, WatershedConfigs,
};
use crate::water::{Water, WaterPresentationAssets};

struct Roots {
	layout: TerrainCellLayout,
	stamps: TerrainStampConfigs,
	watersheds: WatershedConfigs,
	terrain_assets: TerrainPresentationAssets,
	water_assets: WaterPresentationAssets,
}

impl Roots {
	fn new(layout: TerrainCellLayout, seed: u32) -> Self {
		Self {
			layout,
			stamps: TerrainStampConfigs::from_world_seed(seed),
			watersheds: WatershedConfigs::default().with_seed(seed),
			terrain_assets: TerrainPresentationAssets {
				config: TerrainConfig::new(seed),
				material: Handle::default(),
				lod_bands: vec![TerrainMeshLodBand { max_radius_cells: 1, res_2: 2 }],
				outer_add_walls: false,
				fine_grid_max_radius: Some(1),
				macro_seam_half_extents: Vec::new(),
				macro_cell_min_size: None,
				macro_res_2: None,
			},
			water_assets: WaterPresentationAssets { material: Handle::default() },
		}
	}

	fn legacy(&self) -> HcsgStorage {
		let mut storage = HcsgStorage::default();
		register_durham_nodes(&mut storage);
		storage.seed(self.layout.clone(), universal_bounds());
		storage.seed(self.stamps.clone(), universal_bounds());
		storage.seed(self.watersheds.clone(), universal_bounds());
		storage.seed(self.terrain_assets.clone(), universal_bounds());
		storage.seed(self.water_assets.clone(), universal_bounds());
		storage
	}

	fn shared(&self) -> shared::HcsgStorage {
		let storage = shared::HcsgStorage::default();
		DurhamNodes::configure(&storage);
		storage.seed(self.layout.clone(), universal_bounds());
		storage.seed(self.stamps.clone(), universal_bounds());
		storage.seed(self.watersheds.clone(), universal_bounds());
		storage.seed(self.terrain_assets.clone(), universal_bounds());
		storage.seed(self.water_assets.clone(), universal_bounds());
		storage
	}
}

fn sorted(ids: Vec<OriginalId>) -> Vec<Id> {
	let mut ids: Vec<Id> = ids.into_iter().map(|OriginalId(id)| id).collect();
	ids.sort();
	ids
}

/// A 4×4 grid of samples inside `cell`, off the noise lattice.
fn samples(cell: Aabb3d) -> impl Iterator<Item = (f32, f32)> {
	(0..16).map(move |i| {
		let (u, v) = (0.13 + 0.25 * (i % 4) as f32, 0.07 + 0.25 * (i / 4) as f32);
		(cell.min.x + u * (cell.max.x - cell.min.x), cell.min.z + v * (cell.max.z - cell.min.z))
	})
}

/// What legacy picks up from boundary-touching jersey leaves of neighbours it
/// happened to generate first; native reads origins only, so these may differ.
#[derive(Default, Debug)]
struct Divergence {
	jersey_leaves: usize,
	modulations: usize,
	nodes: Vec<&'static str>,
}

fn assert_same_terrain(id: Id, legacy: &Terrain, native: &Terrain, divergence: &mut Divergence) {
	assert_eq!(legacy.cell, native.cell, "{id:?} cell");
	assert_eq!(legacy.res_2, native.res_2, "{id:?} res_2");
	assert_eq!(legacy.stream_ring, native.stream_ring, "{id:?} stream ring");
	assert_eq!(legacy.marazion_fills.len(), native.marazion_fills.len(), "{id:?} fills");
	for (x, z) in samples(legacy.cell) {
		assert_eq!(
			legacy.sdf.terrain().height_at_with_all_modulations(x, z).to_bits(),
			native.sdf.terrain().height_at_with_all_modulations(x, z).to_bits(),
			"{id:?} height at ({x}, {z})"
		);
	}
	let leaves = |terrain: &Terrain| -> Vec<_> {
		terrain
			.marazion_leaves
			.iter()
			.map(|leaf| (leaf.cell, leaf.kind, leaf.band))
			.collect()
	};
	assert_eq!(leaves(legacy), leaves(native), "{id:?} marazion leaves");
	divergence.jersey_leaves += usize::from(legacy.jersey_leaves != native.jersey_leaves);
	divergence.modulations += usize::from(legacy.modulations.len() != native.modulations.len());
}

fn assert_same_water(id: Id, legacy: Option<&Water>, native: Option<&Water>) {
	let (legacy, native) = match (legacy, native) {
		(None, None) => return,
		(Some(legacy), Some(native)) => (legacy, native),
		(legacy, native) => {
			panic!("{id:?} water: legacy {}, native {}", legacy.is_some(), native.is_some())
		}
	};
	assert_eq!(legacy.fills.len(), native.fills.len(), "{id:?} water fills");
	assert_eq!(legacy.res_2, native.res_2, "{id:?} water res_2");
	for (x, z) in samples(legacy.cell) {
		assert_eq!(legacy.column_at(x, z), native.column_at(x, z), "{id:?} column at ({x}, {z})");
	}
}

/// Generates every cell of `roots.layout` both ways and asserts the same
/// terrain heights and water; returns the wet cell count.
fn assert_equivalent(roots: &Roots) -> usize {
	let mut legacy = roots.legacy();
	let storage = roots.shared();
	let mut cx = GenerationContext::new(&storage);
	let region = roots.layout.request_region();

	let ids = sorted(legacy.original_ids_for::<Terrain>(region));
	assert_eq!(ids, sorted(cx.original_ids_for::<Terrain>(region)), "terrain ids");
	assert_eq!(
		sorted(legacy.original_ids_for::<Water>(region)),
		sorted(cx.original_ids_for::<Water>(region)),
		"water ids"
	);
	assert!(!ids.is_empty());

	let mut wet = 0;
	let mut divergence = Divergence::default();
	for &id in &ids {
		let water = legacy.get_one_or_generate::<Water>(id).is_some();
		assert_eq!(water, cx.get_or_generate::<Water>(id).is_some(), "{id:?} water built");
		wet += usize::from(water);
		let native = cx.get_or_generate::<Terrain>(id);
		match (legacy.get_one_or_generate::<Terrain>(id), native) {
			(Some(old), Some(native)) => assert_same_terrain(id, old, &native, &mut divergence),
			(old, native) => {
				panic!("{id:?} terrain: legacy {}, native {}", old.is_some(), native.is_some())
			}
		}
		assert_same_water(id, legacy.get::<Water>(id), storage.get::<Water>(id).as_deref());
	}

	macro_rules! nodes {
		($($T:ty),* $(,)?) => {$(
			let mut a = legacy.overlapping::<$T>(universal_bounds());
			let mut b = storage.overlapping::<$T>(universal_bounds());
			a.sort();
			b.sort();
			if a != b {
				divergence.nodes.push(stringify!($T));
			}
		)*};
	}
	nodes!(
		PreWatershedTerrain,
		HydroComplexCell,
		WatershedCarvingCell,
		WatershedRimmingCell,
		WatershedAproningCell,
		PrePocketLowPassCell,
		PocketHighPassCell,
		PocketWatersLowPass,
		PocketWatersHighPass,
		MassifHighPassControllerCell,
		PlateauHighPassStampCell,
		CanyonLowPassStampCell,
		PocketWaterLowPassStampCell,
		RollingLowPassStampCell,
		ValleyHighPassStampCell,
	);
	println!("{} cells, {wet} wet; {divergence:?}", ids.len());
	wet
}

#[test]
fn a_fine_patch_generates_the_same_both_ways() {
	let mut wet = 0;
	for seed in 1..=3 {
		wet += assert_equivalent(&Roots::new(fine_patch_cell_layout(1, IVec2::new(-1, -1)), seed));
	}
	println!("wet fine cells: {wet}");
}

#[test]
fn a_ringed_layout_generates_the_same_both_ways() {
	let mut layout = fine_patch_cell_layout(1, IVec2::new(-1, -1));
	layout.outer_rings = vec![OuterCellRing { cell_size: 2.0 * TERRAIN_CELL_SIZE, rows: 1 }];
	let wet = assert_equivalent(&Roots::new(layout, 5));
	println!("wet ringed cells: {wet}");
}
