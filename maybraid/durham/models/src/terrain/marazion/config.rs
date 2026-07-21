//! Authoring knobs for Marazion lake leaves.

use crate::terrain::cell::{universal_bounds, TERRAIN_CELL_SIZE};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::lod_ref::LodRef;
use marazion_watersheds::LakeParams;

/// Default lake leaf edge length: 2× terrain cell so rim + apron fit around a body.
pub const DEFAULT_LAKE_LEAF_SIZE: f32 = TERRAIN_CELL_SIZE * 2.0;

/// Universal Marazion watershed configs (lake-first slice).
///
/// Lake look: tune [`Self::lake`]'s authoring knobs
/// (`rim_frac`, `apron_frac`, `water_sink`, `terrain_undercut`) in
/// [`LakeParams::default`](marazion_watersheds::LakeParams::default)
/// or override this resource before generation.
#[derive(Resource, Debug, Clone)]
pub struct MarazionWatershedConfigs {
	pub seed: u32,
	pub lake: LakeParams,
	/// Fraction of lake leaves that stamp.
	pub leaf_likelihood: f32,
	pub spatial_correlation: f32,
	/// Lake leaf edge length (world units); should be ≈2×+ intended body diameter.
	pub leaf_size: f32,
}

impl Default for MarazionWatershedConfigs {
	fn default() -> Self {
		Self {
			seed: 127,
			lake: LakeParams::default(),
			leaf_likelihood: 0.45,
			spatial_correlation: DEFAULT_LAKE_LEAF_SIZE * 0.5,
			leaf_size: DEFAULT_LAKE_LEAF_SIZE,
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
		Some((
			spatial_index.bootstrap_marazion_watershed_configs(),
			universal_bounds(),
		))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
