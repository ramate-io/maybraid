//! Materialized jersey [`StampSet`] for one origin cell.

use crate::terrain::cell::{original_ids_for_origin_cells, TerrainCellLayout};
use crate::terrain::cell_noise::CellTerrainNoise;
use crate::terrain::jersey_plan::JerseyStampPlan;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use jersey_terrain_stamps::{
	Canyon, HydrologyComplex, PlateauCap, PocketWater, RollingGround, RuggedMassif, StampSet,
	ValleyBasin,
};
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use procedural_common::Bounds2;

/// Built jersey stamp output for one terrain origin cell.
#[derive(Debug, Clone, Component)]
pub struct JerseyStamp {
	pub cell: Aabb3d,
	pub plan: JerseyStampPlan,
	pub stamp_set: StampSet,
}

impl JerseyStamp {
	/// Construct the stamp set for `plan` using `noise` as the height oracle.
	pub fn from_plan(cell: Aabb3d, plan: JerseyStampPlan, noise: &CellTerrainNoise) -> Self {
		let bounds = Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z);
		let seed = noise.seed.wrapping_add(cell_salt(cell));
		let height = |x: f32, z: f32| noise.height_at(x, z);
		let height_at: Option<&dyn Fn(f32, f32) -> f32> = Some(&height);
		let stamp_set = match &plan {
			JerseyStampPlan::ValleyBasin(params) => {
				ValleyBasin::from_bounds(bounds, seed, *params, height_at).stamp
			}
			JerseyStampPlan::PlateauCap(params) => {
				PlateauCap::from_bounds(bounds, seed, *params, height_at).stamp
			}
			JerseyStampPlan::RuggedMassif(params) => {
				RuggedMassif::from_bounds(bounds, seed, *params).stamp
			}
			JerseyStampPlan::Canyon(params) => {
				Canyon::from_bounds(bounds, seed, *params, height_at).stamp
			}
			JerseyStampPlan::PocketWater(params) => {
				PocketWater::from_bounds(bounds, seed, *params, height_at).stamp
			}
			JerseyStampPlan::RollingGround(params) => {
				RollingGround::from_bounds(bounds, seed, *params).stamp
			}
			JerseyStampPlan::HydrologyComplex(params) => {
				HydrologyComplex::from_bounds(bounds, seed, *params, height_at).stamp
			}
		};
		Self { cell, plan, stamp_set }
	}
}

fn cell_salt(cell: Aabb3d) -> u32 {
	let ix = cell.min.x.to_bits();
	let iz = cell.min.z.to_bits();
	ix.wrapping_mul(73856093) ^ iz.wrapping_mul(19349663)
}

impl<S> GenerationScheme<S> for JerseyStamp
where
	S: GeneratingSpatialIndex<JerseyStampPlan>
		+ GeneratingSpatialIndex<CellTerrainNoise>
		+ GeneratingSpatialIndex<TerrainCellLayout>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_origin_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		GeneratingSpatialIndex::<JerseyStampPlan>::get_or_generate(spatial_index, id, lod_ref)?;
		let plan = <S as SpatialIndex<JerseyStampPlan>>::get(spatial_index, id)?.clone();
		GeneratingSpatialIndex::<CellTerrainNoise>::get_or_generate(spatial_index, id, lod_ref)?;
		let noise = <S as SpatialIndex<CellTerrainNoise>>::get(spatial_index, id)?;
		Some((Self::from_plan(bounds, plan, noise), bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
