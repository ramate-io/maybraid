//! Watershed correction cells ([`WATERSHED_CORRECTION.md`] bones).
//!
//! Pipeline:
//! ```text
//! PocketWaters{High,Low}Pass (authored enum → HydrologyNodes)
//!   → HydroComplexCell (origin grid: union nodes from both passes)
//!   → CarvingCell / RimmingCell / AproningCell (stage bones)
//!   → Terrain applies HydroComplex (internal carve → rim → apron)
//! ```

use crate::terrain::cell::{original_ids_for_origin_cells, TerrainCellLayout};
use crate::terrain::watersheds::config::WatershedConfigs;
use crate::terrain::watersheds::high_pass::PocketWatersHighPass;
use crate::terrain::watersheds::low_pass::PocketWatersLowPass;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId};
use lod::lod_ref::LodRef;
use procedural_common::Bounds2;
use std::sync::Arc;
use terrain_watersheds::{CorrectionStage, HydroComplex};

/// Origin-grid hydrology complex: unions hydrology nodes from both pocket-water passes.
#[derive(Debug, Clone, Component)]
pub struct HydroComplexCell {
	pub cell: Aabb3d,
	pub complex: Arc<HydroComplex>,
}

impl HydroComplexCell {
	/// Indexed complex when it has hydrology members.
	pub fn indexed(&self) -> Option<&Arc<HydroComplex>> {
		(!self.complex.is_empty()).then_some(&self.complex)
	}
}

fn aabb_to_bounds2(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z)
}

fn cell_seed(cell: Aabb3d, salt: u32) -> u32 {
	salt.wrapping_add(cell.min.x.to_bits().wrapping_mul(73856093))
		.wrapping_add(cell.min.z.to_bits().wrapping_mul(19349663))
}

/// Origin-grid root for watershed correction; both pocket-water passes are
/// pulled by region, so their pocket / pre-pocket stacks stay out of these bounds.
impl<S> GenerationScheme<S> for HydroComplexCell
where
	S: GeneratingSpatialIndex<TerrainCellLayout>
		+ GeneratingSpatialIndex<WatershedConfigs>
		+ GeneratingSpatialIndex<PocketWatersHighPass>
		+ GeneratingSpatialIndex<PocketWatersLowPass>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_origin_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let cell = id.origin_cell_bounds()?;
		let cell_bounds = aabb_to_bounds2(cell);

		let configs = GeneratingSpatialIndex::<WatershedConfigs>::get_one_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let seed = cell_seed(cell, configs.seed);

		let mut hydrology = Vec::new();
		for pass in GeneratingSpatialIndex::<PocketWatersHighPass>::get_or_generate_region_values(
			spatial_index,
			cell,
			lod_ref,
		) {
			hydrology.extend(
				pass.hydro_nodes()
					.into_iter()
					.filter(|node| node.correction_intersects(cell_bounds)),
			);
		}
		for pass in GeneratingSpatialIndex::<PocketWatersLowPass>::get_or_generate_region_values(
			spatial_index,
			cell,
			lod_ref,
		) {
			hydrology.extend(
				pass.hydro_nodes()
					.into_iter()
					.filter(|node| node.correction_intersects(cell_bounds)),
			);
		}

		let complex = Arc::new(HydroComplex::new(cell_bounds, seed).with_hydro(hydrology));

		Some((Self { cell, complex }, cell))
	}
}

/// Origin-cell carve stage over the cellular [`HydroComplexCell`].
#[derive(Debug, Clone, Component)]
pub struct WatershedCarvingCell {
	pub cell: Aabb3d,
	pub complex: Option<Arc<HydroComplex>>,
}

/// Rim correction (raise-only bank toward shelf_anchor + rim_lift).
#[derive(Debug, Clone, Component)]
pub struct WatershedRimmingCell {
	pub cell: Aabb3d,
	pub complex: Option<Arc<HydroComplex>>,
}

/// Apron correction (fade from bank toward identity).
#[derive(Debug, Clone, Component)]
pub struct WatershedAproningCell {
	pub cell: Aabb3d,
	pub complex: Option<Arc<HydroComplex>>,
}

/// Stage cells are views over [`HydroComplexCell`]: same origin ids, and that
/// is their only dependency.
macro_rules! impl_correction_stage_cell {
	($Cell:ty) => {
		impl<S> GenerationScheme<S> for $Cell
		where
			S: GeneratingSpatialIndex<HydroComplexCell>,
		{
			fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
				GeneratingSpatialIndex::<HydroComplexCell>::original_ids_for(spatial_index, region)
			}

			fn build_with_id(
				spatial_index: &mut S,
				id: Id,
				lod_ref: &LodRef,
			) -> Option<(Self, Aabb3d)> {
				let complex_cell = GeneratingSpatialIndex::<HydroComplexCell>::get_one_or_generate(
					spatial_index,
					id,
					lod_ref,
				)?;
				let cell = complex_cell.cell;
				Some((Self { cell, complex: complex_cell.indexed().cloned() }, cell))
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
