//! Authoring knobs for Marazion pocket-water lakes.

use crate::terrain::cell::universal_bounds;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::lod_ref::LodRef;
use marazion_watersheds::{
	LakeParams, PocketGuillotineParams, PrePocketParams, DEFAULT_PRE_POCKET_PITCH,
};

/// Universal Marazion watershed configs (lake-first slice).
///
/// Hierarchy: [`PrePocketParams`] → guillotine leaves → [`LakeParams`].
/// Lake look: keep [`LakeParams::apron_frac`] large; rim/apron claim budget and
/// water takes a noisy fraction of the leftover ([`LakeParams::water_size`] /
/// [`LakeParams::water_size_min`], high-freq [`LakeParams::water_size_freq`]).
/// Tune in [`LakeParams::default`](marazion_watersheds::LakeParams::default).
#[derive(Resource, Debug, Clone)]
pub struct MarazionWatershedConfigs {
	pub seed: u32,
	pub pre_pocket: PrePocketParams,
	pub guillotine: PocketGuillotineParams,
	pub lake: LakeParams,
	/// Fraction of lake leaves that stamp.
	pub leaf_likelihood: f32,
	pub spatial_correlation: f32,
}

impl Default for MarazionWatershedConfigs {
	fn default() -> Self {
		Self {
			seed: 127,
			pre_pocket: PrePocketParams {
				pitch: DEFAULT_PRE_POCKET_PITCH,
				seed: 127,
				..Default::default()
			},
			guillotine: PocketGuillotineParams { seed: 127, ..Default::default() },
			lake: LakeParams::default(),
			leaf_likelihood: 0.65,
			spatial_correlation: DEFAULT_PRE_POCKET_PITCH * 0.25,
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
