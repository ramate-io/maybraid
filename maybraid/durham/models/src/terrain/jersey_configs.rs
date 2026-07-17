//! Universal authoring knobs for independent jersey stamp family layers.

use crate::terrain::cell::universal_bounds;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use jersey_terrain_stamps::{
	CanyonParams, PlateauCapParams, PocketWaterParams, RollingGroundParams, RuggedMassifParams,
};
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::lod_ref::LodRef;

/// Per-family params applied on every jersey stamp cell (layers coexist).
///
/// Valley trains are authored via [`crate::terrain::valley_chain::JerseyValleyChainLayerConfig`],
/// not this grid-family set.
#[derive(Resource, Debug, Clone)]
pub struct JerseyLayerConfigs {
	pub plateau: PlateauCapParams,
	pub massif: RuggedMassifParams,
	pub canyon: CanyonParams,
	pub pocket_water: PocketWaterParams,
	pub rolling: RollingGroundParams,
}

impl Default for JerseyLayerConfigs {
	fn default() -> Self {
		Self {
			plateau: PlateauCapParams::default(),
			massif: RuggedMassifParams::default(),
			canyon: CanyonParams::default(),
			pocket_water: PocketWaterParams::default(),
			rolling: RollingGroundParams::default(),
		}
	}
}

/// Bootstrap source used only when first materializing [`JerseyLayerConfigs`] at
/// [`Id::Universal`].
pub trait BootstrapJerseyLayerConfigs {
	fn bootstrap_jersey_layer_configs(&self) -> JerseyLayerConfigs;
}

impl<S> GenerationScheme<S> for JerseyLayerConfigs
where
	S: BootstrapJerseyLayerConfigs,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		Some((spatial_index.bootstrap_jersey_layer_configs(), universal_bounds()))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
