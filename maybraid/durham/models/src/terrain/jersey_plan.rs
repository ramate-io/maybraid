//! Per-cell stamp family selection from [`CellTerrainNoise`] thresholds.

use crate::terrain::cell::{original_ids_for_origin_cells, TerrainCellLayout};
use crate::terrain::cell_noise::CellTerrainNoise;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use jersey_terrain_stamps::{
	CanyonParams, CanyonVariant, HydrologyComplexParams, PlateauCapParams, PocketWaterParams,
	RollingGroundParams, RuggedMassifParams, ValleyBasinParams, ValleyCrossSection,
	ValleyFloorKind,
};
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;
use procedural_common::SeededHash;

/// Deterministic jersey stamp configs for one origin cell.
#[derive(Debug, Clone, Component)]
pub enum JerseyStampPlan {
	ValleyBasin(ValleyBasinParams),
	PlateauCap(PlateauCapParams),
	RuggedMassif(RuggedMassifParams),
	Canyon(CanyonParams),
	PocketWater(PocketWaterParams),
	RollingGround(RollingGroundParams),
	HydrologyComplex(HydrologyComplexParams),
}

impl JerseyStampPlan {
	/// Choose a primary family from cell noise stats + seeded thresholds.
	pub fn from_cell_noise(noise: &CellTerrainNoise) -> Self {
		let salt = cell_salt(noise.cell);
		let hash = SeededHash::new(noise.seed.wrapping_add(salt));
		let delta = noise.relief_delta();
		let scale = noise.range.max(1.0);
		let normalized_delta = delta / scale;

		// Rare hydrology complex (~4%).
		if hash.unit(0) < 0.04 {
			return Self::HydrologyComplex(HydrologyComplexParams::default());
		}

		if normalized_delta < -0.12 {
			// Depression: valley / canyon / pocket.
			let pick = hash.unit(1);
			if pick < 0.55 {
				let mut params = ValleyBasinParams::default();
				params.depth = (8.0 + hash.unit(2) * 16.0).min(noise.range * 0.35 + 6.0);
				params.width_frac = 0.12 + hash.unit(3) * 0.18;
				params.cross_section = if hash.unit(4) < 0.35 {
					ValleyCrossSection::V
				} else if hash.unit(4) < 0.7 {
					ValleyCrossSection::U
				} else {
					ValleyCrossSection::Asymmetric
				};
				params.floor = if hash.unit(5) < 0.4 {
					ValleyFloorKind::Arroyo
				} else {
					ValleyFloorKind::SpillwayReady
				};
				Self::ValleyBasin(params)
			} else if pick < 0.8 {
				let mut params = CanyonParams::default();
				params.variant = if hash.unit(6) < 0.5 {
					CanyonVariant::Unchained
				} else {
					CanyonVariant::Chained
				};
				Self::Canyon(params)
			} else {
				Self::PocketWater(PocketWaterParams::default())
			}
		} else if normalized_delta > 0.12 {
			// Ridge: massif / plateau.
			if hash.unit(7) < 0.55 {
				Self::RuggedMassif(RuggedMassifParams::default())
			} else {
				Self::PlateauCap(PlateauCapParams::default())
			}
		} else {
			// Flat / mild relief.
			Self::RollingGround(RollingGroundParams::default())
		}
	}
}

fn cell_salt(cell: Aabb3d) -> u32 {
	let ix = cell.min.x.to_bits();
	let iz = cell.min.z.to_bits();
	ix.wrapping_mul(73856093) ^ iz.wrapping_mul(19349663)
}

impl<S> GenerationScheme<S> for JerseyStampPlan
where
	S: GeneratingSpatialIndex<CellTerrainNoise> + GeneratingSpatialIndex<TerrainCellLayout>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		original_ids_for_origin_cells(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		GeneratingSpatialIndex::<CellTerrainNoise>::get_or_generate(spatial_index, id, lod_ref)?;
		let noise = <S as SpatialIndex<CellTerrainNoise>>::get(spatial_index, id)?;
		Some((Self::from_cell_noise(noise), bounds))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
