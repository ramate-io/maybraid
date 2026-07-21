//! Authoring knobs for Marazion pocket-water lakes (dual-band).

use crate::terrain::cell::universal_bounds;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::*;
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::lod_ref::LodRef;
use marazion_watersheds::{
	LakeParams, PocketGuillotineParams, PrePocketParams, DEFAULT_POCKET_PITCHES_HIGH,
	DEFAULT_POCKET_PITCHES_LOW, DEFAULT_PRE_POCKET_PITCH, DEFAULT_PRE_POCKET_PITCH_LOW,
};

/// One occupancy / scale band (low-pass = small, high-pass = large).
#[derive(Debug, Clone)]
pub struct MarazionBandConfig {
	pub pre_pocket: PrePocketParams,
	pub guillotine: PocketGuillotineParams,
	pub lake: LakeParams,
	pub leaf_likelihood: f32,
	pub spatial_correlation: f32,
	pub family_salt: u32,
}

impl MarazionBandConfig {
	/// Small lakes: leaf sides ≈ **200–600m**.
	pub fn low_pass_default() -> Self {
		let mut lake = LakeParams::default();
		lake.depth = 11.0;
		lake.water_scale_min = 0.45;
		Self {
			pre_pocket: PrePocketParams {
				pitch: DEFAULT_PRE_POCKET_PITCH_LOW,
				origin: Vec2::new(187.0, 93.0),
				pocket_pitches: DEFAULT_POCKET_PITCHES_LOW,
				seed: 127,
			},
			guillotine: PocketGuillotineParams {
				max_depth: 3,
				min_span: 200.0,
				seed: 127,
				..Default::default()
			},
			lake,
			leaf_likelihood: 0.28,
			spatial_correlation: DEFAULT_PRE_POCKET_PITCH_LOW * 0.5,
			family_salt: 0x1270_0001,
		}
	}

	/// Large lakes: leaf sides ≈ **800m–3km**.
	pub fn high_pass_default() -> Self {
		let mut lake = LakeParams::default();
		lake.depth = 18.0;
		lake.water_scale_min = 0.40;
		Self {
			pre_pocket: PrePocketParams {
				pitch: DEFAULT_PRE_POCKET_PITCH,
				origin: Vec2::new(640.0, 1280.0),
				pocket_pitches: DEFAULT_POCKET_PITCHES_HIGH,
				seed: 127,
			},
			guillotine: PocketGuillotineParams {
				max_depth: 3,
				min_span: 800.0,
				seed: 127,
				..Default::default()
			},
			lake,
			leaf_likelihood: 0.14,
			spatial_correlation: DEFAULT_PRE_POCKET_PITCH * 2.0,
			family_salt: 0x1270_0002,
		}
	}
}

/// Universal Marazion configs: parallel low-pass + high-pass stacks.
#[derive(Resource, Debug, Clone)]
pub struct MarazionWatershedConfigs {
	pub seed: u32,
	pub low_pass: MarazionBandConfig,
	pub high_pass: MarazionBandConfig,
}

impl Default for MarazionWatershedConfigs {
	fn default() -> Self {
		let mut low = MarazionBandConfig::low_pass_default();
		let mut high = MarazionBandConfig::high_pass_default();
		low.pre_pocket.seed = 127;
		low.guillotine.seed = 127;
		high.pre_pocket.seed = 127;
		high.guillotine.seed = 127;
		Self {
			seed: 127,
			low_pass: low,
			high_pass: high,
		}
	}
}

pub trait BootstrapMarazionWatershedConfigs {
	fn bootstrap_marazion_watershed_configs(&self) -> MarazionWatershedConfigs;
}

impl<S> GenerationScheme<S> for MarazionWatershedConfigs
where
	S: BootstrapMarazionWatershedConfigs,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		Some((spatial_index.bootstrap_marazion_watershed_configs(), universal_bounds()))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
