//! Universal per-family guillotine + stamp authoring knobs (dual band).

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

/// Guillotine cut knobs + stamp params + occupancy for one family band.
#[derive(Debug, Clone)]
pub struct FamilyGuillotineConfig<P> {
	pub seed: u32,
	pub depth: u8,
	pub guillotine: GuillotineConfig,
	pub noise_frequency: f32,
	/// World-space frequency for occupancy Perlin (spatial correlation scale).
	pub occupancy_frequency: f32,
	/// Soft threshold on occupancy noise (`0.0..=1.0`); higher → more leaves accepted.
	pub likelihood: f32,
	pub stamp: P,
}

impl<P: Default> FamilyGuillotineConfig<P> {
	fn low_pass(seed: u32, likelihood: f32) -> Self {
		Self {
			seed,
			depth: 6,
			guillotine: GuillotineConfig::new(TERRAIN_CELL_SIZE * 1.25, MACRO_CELL_SIZE * 1.5)
				.with_snap_quantum(20.0),
			noise_frequency: 0.05,
			// Correlation on the order of a few low-pass controller cells.
			occupancy_frequency: 1.0 / (MACRO_CELL_SIZE * 12.0),
			likelihood: likelihood.clamp(0.0, 1.0),
			stamp: P::default(),
		}
	}

	fn high_pass(seed: u32, likelihood: f32) -> Self {
		Self {
			seed,
			depth: 4,
			// Large preferred leaves for regional features.
			guillotine: GuillotineConfig::new(MACRO_CELL_SIZE * 2.0, MACRO_CELL_SIZE * 8.0)
				.with_snap_quantum(40.0),
			noise_frequency: 0.02,
			// Broader regional occupancy blobs.
			occupancy_frequency: 1.0 / (MACRO_CELL_SIZE * 80.0),
			likelihood: likelihood.clamp(0.0, 1.0),
			stamp: P::default(),
		}
	}
}

/// Low-pass (detail) + high-pass (regional) knobs for one stamp family.
#[derive(Debug, Clone)]
pub struct DualBandFamilyConfig<P> {
	pub low_pass: FamilyGuillotineConfig<P>,
	pub high_pass: FamilyGuillotineConfig<P>,
}

/// Universal configs for every jersey family's dual-band guillotine grids.
#[derive(Resource, Debug, Clone)]
pub struct JerseyStampConfigs {
	pub plateau: DualBandFamilyConfig<PlateauCapParams>,
	pub massif: DualBandFamilyConfig<RuggedMassifParams>,
	pub canyon: DualBandFamilyConfig<CanyonParams>,
	pub pocket_water: DualBandFamilyConfig<PocketWaterParams>,
	pub rolling: DualBandFamilyConfig<RollingGroundParams>,
	pub valley: DualBandFamilyConfig<ValleyTrainParams>,
}

impl Default for JerseyStampConfigs {
	fn default() -> Self {
		Self {
			plateau: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(42, 0.82),
				high_pass: FamilyGuillotineConfig::high_pass(1042, 0.28),
			},
			massif: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(43, 0.78),
				high_pass: FamilyGuillotineConfig::high_pass(1043, 0.24),
			},
			canyon: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(44, 0.78),
				high_pass: FamilyGuillotineConfig::high_pass(1044, 0.24),
			},
			pocket_water: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(45, 0.88),
				high_pass: FamilyGuillotineConfig::high_pass(1045, 0.2),
			},
			rolling: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(46, 0.92),
				high_pass: FamilyGuillotineConfig::high_pass(1046, 0.35),
			},
			valley: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(47, 0.85),
				high_pass: FamilyGuillotineConfig::high_pass(1047, 0.28),
			},
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
