//! Universal per-family guillotine + stamp authoring knobs (dual band).

use crate::terrain::cell::universal_bounds;
use crate::terrain::jersey::canyon::{
	CanyonHighPassControllerLayout, CanyonLowPassControllerLayout,
};
use crate::terrain::jersey::massif::{
	MassifHighPassControllerLayout, MassifLowPassControllerLayout,
};
use crate::terrain::jersey::plateau::{
	PlateauHighPassControllerLayout, PlateauLowPassControllerLayout,
};
use crate::terrain::jersey::pocket_water::{
	PocketWaterHighPassControllerLayout, PocketWaterLowPassControllerLayout,
};
use crate::terrain::jersey::rolling::{
	RollingHighPassControllerLayout, RollingLowPassControllerLayout,
};
use crate::terrain::jersey::valley::{
	ValleyHighPassControllerLayout, ValleyLowPassControllerLayout,
};
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
	/// Occupancy value-noise lattice spacing (world units) — spatial correlation length.
	pub spatial_correlation: f32,
	/// Approximate leaf acceptance rate (`0.0..=1.0`) for value-noise occupancy.
	///
	/// Prefer setting defaults in `define_jersey_family!` (`likelihood:`); this
	/// field is the runtime override on the resource.
	pub likelihood: f32,
	/// Per-leaf stamp strength lower bound (`1.0` ≈ default vertical knobs).
	pub strength_min: f32,
	/// Per-leaf stamp strength upper bound.
	pub strength_max: f32,
	pub stamp: P,
}

impl<P: Default> FamilyGuillotineConfig<P> {
	fn low_pass(
		seed: u32,
		likelihood: f32,
		spatial_correlation: f32,
		strength_min: f32,
		strength_max: f32,
		cell_size_min: f32,
		cell_size_max: f32,
	) -> Self {
		Self {
			seed,
			depth: 6,
			guillotine: GuillotineConfig::new(cell_size_min, cell_size_max).with_snap_quantum(20.0),
			noise_frequency: 0.05,
			spatial_correlation,
			likelihood: likelihood.clamp(0.0, 1.0),
			strength_min: strength_min.max(0.0),
			strength_max: strength_max.max(0.0),
			stamp: P::default(),
		}
	}

	fn high_pass(
		seed: u32,
		likelihood: f32,
		spatial_correlation: f32,
		strength_min: f32,
		strength_max: f32,
		cell_size_min: f32,
		cell_size_max: f32,
	) -> Self {
		Self {
			seed,
			depth: 4,
			guillotine: GuillotineConfig::new(cell_size_min, cell_size_max).with_snap_quantum(40.0),
			noise_frequency: 0.02,
			spatial_correlation,
			likelihood: likelihood.clamp(0.0, 1.0),
			strength_min: strength_min.max(0.0),
			strength_max: strength_max.max(0.0),
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

macro_rules! band_from_layout {
	(low_pass, $seed:expr, $Layout:ty) => {
		FamilyGuillotineConfig::low_pass(
			$seed,
			<$Layout>::LIKELIHOOD,
			<$Layout>::SPATIAL_CORRELATION,
			<$Layout>::STRENGTH_MIN,
			<$Layout>::STRENGTH_MAX,
			<$Layout>::CELL_SIZE_MIN,
			<$Layout>::CELL_SIZE_MAX,
		)
	};
	(high_pass, $seed:expr, $Layout:ty) => {
		FamilyGuillotineConfig::high_pass(
			$seed,
			<$Layout>::LIKELIHOOD,
			<$Layout>::SPATIAL_CORRELATION,
			<$Layout>::STRENGTH_MIN,
			<$Layout>::STRENGTH_MAX,
			<$Layout>::CELL_SIZE_MIN,
			<$Layout>::CELL_SIZE_MAX,
		)
	};
}

impl Default for JerseyStampConfigs {
	fn default() -> Self {
		// Defaults come from each band's layout consts (`define_jersey_family!`).
		Self {
			plateau: DualBandFamilyConfig {
				low_pass: band_from_layout!(low_pass, 42, PlateauLowPassControllerLayout),
				high_pass: band_from_layout!(high_pass, 1042, PlateauHighPassControllerLayout),
			},
			massif: DualBandFamilyConfig {
				low_pass: band_from_layout!(low_pass, 43, MassifLowPassControllerLayout),
				high_pass: band_from_layout!(high_pass, 1043, MassifHighPassControllerLayout),
			},
			canyon: DualBandFamilyConfig {
				low_pass: band_from_layout!(low_pass, 44, CanyonLowPassControllerLayout),
				high_pass: band_from_layout!(high_pass, 1044, CanyonHighPassControllerLayout),
			},
			pocket_water: DualBandFamilyConfig {
				low_pass: band_from_layout!(low_pass, 45, PocketWaterLowPassControllerLayout),
				high_pass: band_from_layout!(high_pass, 1045, PocketWaterHighPassControllerLayout),
			},
			rolling: DualBandFamilyConfig {
				low_pass: band_from_layout!(low_pass, 46, RollingLowPassControllerLayout),
				high_pass: band_from_layout!(high_pass, 1046, RollingHighPassControllerLayout),
			},
			valley: DualBandFamilyConfig {
				low_pass: band_from_layout!(low_pass, 47, ValleyLowPassControllerLayout),
				high_pass: band_from_layout!(high_pass, 1047, ValleyHighPassControllerLayout),
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
		Some((spatial_index.bootstrap_jersey_stamp_configs(), universal_bounds()))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
