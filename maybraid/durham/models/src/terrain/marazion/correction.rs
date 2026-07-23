//! Watershed correction cells ([`WATERSHED_CORRECTION.md`] bones).
//!
//! Pipeline:
//! ```text
//! MarazionPocketWaters{High,Low}Pass (authored enum → HydrologyNodes)
//!   → WatershedDepressionComplexCell (origin grid: union nodes from both passes)
//!   → CarvingCell / RimmingCell / AproningCell
//!   → Terrain (carve → rim → apron)
//! ```

use crate::terrain::cell::original_ids_for_origin_cells;
use crate::terrain::marazion::high_pass::{MarazionPocketWatersHighPass, PocketHighPassCell};
use crate::terrain::marazion::low_pass::{MarazionPocketWatersLowPass, PocketLowPassCell};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId};
use lod::lod_ref::LodRef;
use marazion_watersheds::{
	CorrectionStage, PreparedHydroComplex, WatershedDepressionComplex,
};
use procedural_common::Bounds2;

/// Origin-grid watershed complex: unions hydrology nodes from both pocket-water passes.
#[derive(Debug, Clone, Component)]
pub struct WatershedDepressionComplexCell {
	pub cell: Aabb3d,
	pub complex: WatershedDepressionComplex,
}

impl WatershedDepressionComplexCell {
	pub fn compile_prepared(&self) -> Option<PreparedHydroComplex> {
		self.complex.compile().hydro
	}
}

fn aabb_to_bounds2(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z)
}

fn cell_seed(cell: Aabb3d, salt: u32) -> u32 {
	salt.wrapping_add(cell.min.x.to_bits().wrapping_mul(73856093))
		.wrapping_add(cell.min.z.to_bits().wrapping_mul(19349663))
}

impl<S> GenerationScheme<S> for WatershedDepressionComplexCell
where
	S: GeneratingSpatialIndex<MarazionPocketWatersHighPass>
		+ GeneratingSpatialIndex<MarazionPocketWatersLowPass>
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

		let configs = GeneratingSpatialIndex::<
			crate::terrain::marazion::config::MarazionWatershedConfigs,
		>::get_one_or_generate(spatial_index, Id::Universal, lod_ref)?;
		let seed = cell_seed(cell, configs.seed);

		let mut hydrology = Vec::new();
		for pass in GeneratingSpatialIndex::<MarazionPocketWatersHighPass>::get_or_generate_region_values(
			spatial_index,
			cell,
			lod_ref,
		) {
			hydrology.extend(pass.hydrology_nodes());
		}
		for pass in GeneratingSpatialIndex::<MarazionPocketWatersLowPass>::get_or_generate_region_values(
			spatial_index,
			cell,
			lod_ref,
		) {
			hydrology.extend(pass.hydrology_nodes());
		}

		let complex =
			WatershedDepressionComplex::new(cell_bounds, seed).with_hydrology(hydrology);

		Some((Self { cell, complex }, cell))
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
		+ GeneratingSpatialIndex<MarazionPocketWatersHighPass>
		+ GeneratingSpatialIndex<MarazionPocketWatersLowPass>
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
				+ GeneratingSpatialIndex<MarazionPocketWatersHighPass>
				+ GeneratingSpatialIndex<MarazionPocketWatersLowPass>
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
