//! Universal per-family guillotine + stamp authoring knobs.

use crate::terrain::cell::{universal_bounds, MACRO_CELL_SIZE, TERRAIN_CELL_SIZE};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use comproc::guillotine::GuillotineConfig;
use jersey_terrain_stamps::{
	CanyonParams, PlateauCapParams, PocketWaterParams, RollingGroundParams, RuggedMassifParams,
	ValleyTrainParams,
};
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::lod_ref::LodRef;

/// Guillotine cut knobs + stamp params for one jersey family.
#[derive(Debug, Clone)]
pub struct FamilyGuillotineConfig<P> {
	pub seed: u32,
	pub depth: u8,
	pub guillotine: GuillotineConfig,
	pub noise_frequency: f32,
	pub stamp: P,
}

impl<P: Default> FamilyGuillotineConfig<P> {
	fn with_seed(seed: u32) -> Self {
		Self {
			seed,
			depth: 8,
			guillotine: GuillotineConfig::new(TERRAIN_CELL_SIZE, MACRO_CELL_SIZE)
				.with_snap_quantum(20.0),
			noise_frequency: 0.05,
			stamp: P::default(),
		}
	}
}

/// Universal configs for every jersey family's independent guillotine grid.
#[derive(Resource, Debug, Clone)]
pub struct JerseyStampConfigs {
	pub plateau: FamilyGuillotineConfig<PlateauCapParams>,
	pub massif: FamilyGuillotineConfig<RuggedMassifParams>,
	pub canyon: FamilyGuillotineConfig<CanyonParams>,
	pub pocket_water: FamilyGuillotineConfig<PocketWaterParams>,
	pub rolling: FamilyGuillotineConfig<RollingGroundParams>,
	pub valley: FamilyGuillotineConfig<ValleyTrainParams>,
}

impl Default for JerseyStampConfigs {
	fn default() -> Self {
		Self {
			plateau: FamilyGuillotineConfig::with_seed(42),
			massif: FamilyGuillotineConfig::with_seed(43),
			canyon: FamilyGuillotineConfig::with_seed(44),
			pocket_water: FamilyGuillotineConfig::with_seed(45),
			rolling: FamilyGuillotineConfig::with_seed(46),
			valley: FamilyGuillotineConfig::with_seed(47),
		}
	}
}

/// Bootstrap source for [`JerseyStampConfigs`] at [`Id::Universal`].
pub trait BootstrapJerseyStampConfigs {
	fn bootstrap_jersey_stamp_configs(&self) -> JerseyStampConfigs;
}

impl<S> GenerationScheme<S> for JerseyStampConfigs
where
	S: BootstrapJerseyStampConfigs,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		Some((
			spatial_index.bootstrap_jersey_stamp_configs(),
			universal_bounds(),
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
