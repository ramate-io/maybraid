//! Universal per-family guillotine + stamp authoring knobs (dual band).

use crate::terrain::cell::{universal_bounds, MACRO_CELL_SIZE, TERRAIN_CELL_SIZE};
use crate::terrain::jersey::canyon::{CanyonHighPassControllerLayout, CanyonLowPassControllerLayout};
use crate::terrain::jersey::massif::{MassifHighPassControllerLayout, MassifLowPassControllerLayout};
use crate::terrain::jersey::plateau::{
	PlateauHighPassControllerLayout, PlateauLowPassControllerLayout,
};
use crate::terrain::jersey::pocket_water::{
	PocketWaterHighPassControllerLayout, PocketWaterLowPassControllerLayout,
};
use crate::terrain::jersey::rolling::{
	RollingHighPassControllerLayout, RollingLowPassControllerLayout,
};
use crate::terrain::jersey::valley::{ValleyHighPassControllerLayout, ValleyLowPassControllerLayout};
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
	/// World-space frequency of the occupancy value-noise lattice.
	pub occupancy_frequency: f32,
	/// Approximate leaf acceptance rate (`0.0..=1.0`) for value-noise occupancy.
	///
	/// Prefer setting defaults in `define_jersey_family!` (`likelihood:`); this
	/// field is the runtime override on the resource.
	pub likelihood: f32,
	pub stamp: P,
}

impl<P: Default> FamilyGuillotineConfig<P> {
	fn low_pass(seed: u32, likelihood: f32, occupancy_frequency: f32) -> Self {
		Self {
			seed,
			depth: 6,
			guillotine: GuillotineConfig::new(TERRAIN_CELL_SIZE * 1.25, MACRO_CELL_SIZE * 1.5)
				.with_snap_quantum(20.0),
			noise_frequency: 0.05,
			occupancy_frequency,
			likelihood: likelihood.clamp(0.0, 1.0),
			stamp: P::default(),
		}
	}

	fn high_pass(seed: u32, likelihood: f32, occupancy_frequency: f32) -> Self {
		Self {
			seed,
			depth: 4,
			guillotine: GuillotineConfig::new(MACRO_CELL_SIZE * 2.0, MACRO_CELL_SIZE * 8.0)
				.with_snap_quantum(40.0),
			noise_frequency: 0.02,
			occupancy_frequency,
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
		// Likelihood / occupancy_frequency defaults come from each band's layout
		// consts (set in `define_jersey_family!`).
		Self {
			plateau: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(
					42,
					PlateauLowPassControllerLayout::LIKELIHOOD,
					PlateauLowPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
				high_pass: FamilyGuillotineConfig::high_pass(
					1042,
					PlateauHighPassControllerLayout::LIKELIHOOD,
					PlateauHighPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
			},
			massif: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(
					43,
					MassifLowPassControllerLayout::LIKELIHOOD,
					MassifLowPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
				high_pass: FamilyGuillotineConfig::high_pass(
					1043,
					MassifHighPassControllerLayout::LIKELIHOOD,
					MassifHighPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
			},
			canyon: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(
					44,
					CanyonLowPassControllerLayout::LIKELIHOOD,
					CanyonLowPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
				high_pass: FamilyGuillotineConfig::high_pass(
					1044,
					CanyonHighPassControllerLayout::LIKELIHOOD,
					CanyonHighPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
			},
			pocket_water: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(
					45,
					PocketWaterLowPassControllerLayout::LIKELIHOOD,
					PocketWaterLowPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
				high_pass: FamilyGuillotineConfig::high_pass(
					1045,
					PocketWaterHighPassControllerLayout::LIKELIHOOD,
					PocketWaterHighPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
			},
			rolling: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(
					46,
					RollingLowPassControllerLayout::LIKELIHOOD,
					RollingLowPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
				high_pass: FamilyGuillotineConfig::high_pass(
					1046,
					RollingHighPassControllerLayout::LIKELIHOOD,
					RollingHighPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
			},
			valley: DualBandFamilyConfig {
				low_pass: FamilyGuillotineConfig::low_pass(
					47,
					ValleyLowPassControllerLayout::LIKELIHOOD,
					ValleyLowPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
				high_pass: FamilyGuillotineConfig::high_pass(
					1047,
					ValleyHighPassControllerLayout::LIKELIHOOD,
					ValleyHighPassControllerLayout::OCCUPANCY_FREQUENCY,
				),
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
