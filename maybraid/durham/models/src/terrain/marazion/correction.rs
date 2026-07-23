//! Watershed correction cells ([`WATERSHED_CORRECTION.md`] bones).
//!
//! Pipeline:
//! ```text
//! MarazionLake*Cell (banded authored stamps → HydrologyNodes)
//!   → WatershedDepressionComplexCell (origin grid: union nodes from all bands)
//!   → CarvingCell / RimmingCell / AproningCell (same cell → staged correction)
//!   → Terrain (carve → rim → apron)
//! ```

use crate::terrain::cell::original_ids_for_origin_cells;
use crate::terrain::marazion::high_pass::{
	original_ids_for_marazion_lake_high_pass_leaves, MarazionLakeHighPassCell, PocketHighPassCell,
};
use crate::terrain::marazion::leaf_kind::{MarazionBandPass, MarazionLeafBounds};
use crate::terrain::marazion::low_pass::{
	original_ids_for_marazion_lake_low_pass_leaves, MarazionLakeLowPassCell, PocketLowPassCell,
};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId};
use lod::lod_ref::LodRef;
use marazion_watersheds::{
	CorrectionStage, HydrologyNode, PreparedHydroComplex, WatershedBackfill,
	WatershedDepressionComplex,
};
use procedural_common::Bounds2;

/// Origin-grid watershed complex: one cell unions hydrology nodes from every
/// intersecting authored stamp (high-pass and low-pass together).
#[derive(Debug, Clone, Component)]
pub struct WatershedDepressionComplexCell {
	pub cell: Aabb3d,
	pub complex: WatershedDepressionComplex,
	/// Authored leaf bounds that contributed candidates (debug / HUD).
	pub source_leaves: Vec<MarazionLeafBounds>,
}

impl WatershedDepressionComplexCell {
	pub fn compile_prepared(&self) -> Option<PreparedHydroComplex> {
		self.complex.compile().hydro
	}
}

fn aabb_to_bounds2(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z)
}

fn expand_aabb_xz(cell: Aabb3d, pad: f32) -> Aabb3d {
	let pad = pad.max(0.0);
	Aabb3d::from_min_max(
		Vec3::new(cell.min.x - pad, cell.min.y, cell.min.z - pad),
		Vec3::new(cell.max.x + pad, cell.max.y, cell.max.z + pad),
	)
}

fn bounds2_intersects(a: &Bounds2, b: &Bounds2) -> bool {
	a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.y <= b.max.y && a.max.y >= b.min.y
}

fn cell_seed(cell: Aabb3d, salt: u32) -> u32 {
	salt.wrapping_add(cell.min.x.to_bits().wrapping_mul(73856093))
		.wrapping_add(cell.min.z.to_bits().wrapping_mul(19349663))
}

fn take_nodes_intersecting(
	nodes: &[HydrologyNode],
	cell_bounds: &Bounds2,
	out: &mut Vec<HydrologyNode>,
) {
	for node in nodes {
		if bounds2_intersects(&node.correction_index_bounds(), cell_bounds) {
			out.push(node.clone());
		}
	}
}

impl<S> GenerationScheme<S> for WatershedDepressionComplexCell
where
	S: GeneratingSpatialIndex<MarazionLakeHighPassCell>
		+ GeneratingSpatialIndex<MarazionLakeLowPassCell>
		+ GeneratingSpatialIndex<PocketHighPassCell>
		+ GeneratingSpatialIndex<PocketLowPassCell>
		+ GeneratingSpatialIndex<crate::terrain::marazion::config::MarazionWatershedConfigs>
		+ GeneratingSpatialIndex<crate::terrain::PreWatershedTerrain>
		+ GeneratingSpatialIndex<crate::terrain::cell::TerrainCellLayout>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_origin_cells(spatial_index, region)
	}

	fn build_with_id(
		spatial_index: &mut S,
		id: Id,
		lod_ref: &LodRef,
	) -> Option<(Self, Aabb3d)> {
		let cell = id.origin_cell_bounds()?;
		let cell_bounds = aabb_to_bounds2(cell);
		// Pad leaf discovery so nodes whose correction support reaches this cell
		// are loadable even when their authored leaf AABB sits next door.
		let pad = (cell.max.x - cell.min.x)
			.max(cell.max.z - cell.min.z)
			.max(1.0)
			* 0.5;
		let query = expand_aabb_xz(cell, pad);

		let configs = GeneratingSpatialIndex::<
			crate::terrain::marazion::config::MarazionWatershedConfigs,
		>::get_one_or_generate(spatial_index, Id::Universal, lod_ref)?;
		let seed = cell_seed(cell, configs.seed);

		let mut hydrology = Vec::new();
		let mut backfills: Vec<WatershedBackfill> = Vec::new();
		let mut source_leaves = Vec::new();

		let mut high_ids = original_ids_for_marazion_lake_high_pass_leaves(spatial_index, query);
		high_ids.sort_by(|a, b| a.0.cmp(&b.0));
		for OriginalId(lid) in high_ids {
			let leaf = GeneratingSpatialIndex::<MarazionLakeHighPassCell>::get_one_or_generate(
				spatial_index,
				lid,
				lod_ref,
			)?;
			source_leaves.push(MarazionLeafBounds {
				cell: leaf.cell,
				kind: leaf.kind,
				band: MarazionBandPass::High,
			});
			take_nodes_intersecting(&leaf.complex.hydrology, &cell_bounds, &mut hydrology);
			let leaf_b = aabb_to_bounds2(leaf.cell);
			if bounds2_intersects(&leaf_b, &cell_bounds) {
				backfills.extend(leaf.complex.backfills.iter().cloned());
			}
		}

		let mut low_ids = original_ids_for_marazion_lake_low_pass_leaves(spatial_index, query);
		low_ids.sort_by(|a, b| a.0.cmp(&b.0));
		for OriginalId(lid) in low_ids {
			let leaf = GeneratingSpatialIndex::<MarazionLakeLowPassCell>::get_one_or_generate(
				spatial_index,
				lid,
				lod_ref,
			)?;
			source_leaves.push(MarazionLeafBounds {
				cell: leaf.cell,
				kind: leaf.kind,
				band: MarazionBandPass::Low,
			});
			take_nodes_intersecting(&leaf.complex.hydrology, &cell_bounds, &mut hydrology);
			let leaf_b = aabb_to_bounds2(leaf.cell);
			if bounds2_intersects(&leaf_b, &cell_bounds) {
				backfills.extend(leaf.complex.backfills.iter().cloned());
			}
		}

		let mut complex =
			WatershedDepressionComplex::new(cell_bounds, seed).with_hydrology(hydrology);
		for backfill in backfills {
			complex = complex.with_backfill(backfill);
		}

		Some((
			Self {
				cell,
				complex,
				source_leaves,
			},
			cell,
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}

/// Origin-cell carve stage over the cellular [`WatershedDepressionComplexCell`].
#[derive(Debug, Clone, Component)]
pub struct WatershedCarvingCell {
	pub cell: Aabb3d,
	pub prepared: Option<PreparedHydroComplex>,
}

/// Rim correction (raise-only bank toward shelf_anchor + rim_lift).
#[derive(Debug, Clone, Component)]
pub struct WatershedRimmingCell {
	pub cell: Aabb3d,
	pub prepared: Option<PreparedHydroComplex>,
}

/// Apron correction (fade from bank toward identity).
#[derive(Debug, Clone, Component)]
pub struct WatershedAproningCell {
	pub cell: Aabb3d,
	pub prepared: Option<PreparedHydroComplex>,
}

fn prepared_from_complex_cell<S>(
	spatial_index: &mut S,
	id: Id,
	lod_ref: &LodRef,
) -> Option<(Aabb3d, Option<PreparedHydroComplex>)>
where
	S: GeneratingSpatialIndex<WatershedDepressionComplexCell>
		+ GeneratingSpatialIndex<MarazionLakeHighPassCell>
		+ GeneratingSpatialIndex<MarazionLakeLowPassCell>
		+ GeneratingSpatialIndex<PocketHighPassCell>
		+ GeneratingSpatialIndex<PocketLowPassCell>
		+ GeneratingSpatialIndex<crate::terrain::marazion::config::MarazionWatershedConfigs>
		+ GeneratingSpatialIndex<crate::terrain::PreWatershedTerrain>
		+ GeneratingSpatialIndex<crate::terrain::cell::TerrainCellLayout>,
{
	let complex_cell =
		GeneratingSpatialIndex::<WatershedDepressionComplexCell>::get_one_or_generate(
			spatial_index,
			id,
			lod_ref,
		)?;
	Some((complex_cell.cell, complex_cell.compile_prepared()))
}

macro_rules! impl_correction_stage_cell {
	($Cell:ty) => {
		impl<S> GenerationScheme<S> for $Cell
		where
			S: GeneratingSpatialIndex<WatershedDepressionComplexCell>
				+ GeneratingSpatialIndex<MarazionLakeHighPassCell>
				+ GeneratingSpatialIndex<MarazionLakeLowPassCell>
				+ GeneratingSpatialIndex<PocketHighPassCell>
				+ GeneratingSpatialIndex<PocketLowPassCell>
				+ GeneratingSpatialIndex<crate::terrain::marazion::config::MarazionWatershedConfigs>
				+ GeneratingSpatialIndex<crate::terrain::PreWatershedTerrain>
				+ GeneratingSpatialIndex<crate::terrain::cell::TerrainCellLayout>,
		{
			fn original_ids_for(
				spatial_index: &mut S,
				region: Aabb3d,
			) -> Vec<OriginalId> {
				original_ids_for_origin_cells(spatial_index, region)
			}

			fn build_with_id(
				spatial_index: &mut S,
				id: Id,
				lod_ref: &LodRef,
			) -> Option<(Self, Aabb3d)> {
				let (cell, prepared) = prepared_from_complex_cell(spatial_index, id, lod_ref)?;
				Some((Self { cell, prepared }, cell))
			}

			fn descendants_with_lod(
				_id: Id,
				_spatial_index: &mut S,
				_lod_ref: &LodRef,
			) {
			}
		}
	};
}

impl_correction_stage_cell!(WatershedCarvingCell);
impl_correction_stage_cell!(WatershedRimmingCell);
impl_correction_stage_cell!(WatershedAproningCell);

impl WatershedCarvingCell {
	pub const STAGE: CorrectionStage = CorrectionStage::Carve;
}

impl WatershedRimmingCell {
	pub const STAGE: CorrectionStage = CorrectionStage::Rim;
}

impl WatershedAproningCell {
	pub const STAGE: CorrectionStage = CorrectionStage::Apron;
}
