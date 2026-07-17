//! Universal authoring knobs for the ValleyChain guillotine hierarchy.

use crate::terrain::cell::{universal_bounds, MACRO_CELL_SIZE, TERRAIN_CELL_SIZE};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use comproc::guillotine::GuillotineConfig;
use jersey_terrain_stamps::ValleyTrainParams;
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::lod_ref::LodRef;

/// Per-world params for ValleyChain controller cuts and leaf stamps.
#[derive(Resource, Debug, Clone)]
pub struct JerseyValleyChainLayerConfig {
	/// Seed salt for guillotine noise (combined with controller cell salt).
	pub seed: u32,
	/// Guillotine cut-attempt depth.
	pub depth: u8,
	/// Preferred leaf step window / snap for middle-out cuts.
	pub guillotine: GuillotineConfig,
	/// Noise frequency for cut sampling.
	pub noise_frequency: f32,
	/// Leaf stamp params (`ValleyTrain`).
	pub valley_train: ValleyTrainParams,
}

impl Default for JerseyValleyChainLayerConfig {
	fn default() -> Self {
		Self {
			seed: 42,
			depth: 8,
			guillotine: GuillotineConfig::new(TERRAIN_CELL_SIZE, MACRO_CELL_SIZE)
				.with_snap_quantum(20.0),
			noise_frequency: 0.05,
			valley_train: ValleyTrainParams::default(),
		}
	}
}

/// Bootstrap source used only when first materializing
/// [`JerseyValleyChainLayerConfig`] at [`Id::Universal`].
pub trait BootstrapJerseyValleyChainLayerConfig {
	fn bootstrap_jersey_valley_chain_layer_config(&self) -> JerseyValleyChainLayerConfig;
}

impl<S> GenerationScheme<S> for JerseyValleyChainLayerConfig
where
	S: BootstrapJerseyValleyChainLayerConfig,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		Some((
			spatial_index.bootstrap_jersey_valley_chain_layer_config(),
			universal_bounds(),
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
