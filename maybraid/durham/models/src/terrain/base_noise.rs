//! Universal base terrain noise — one value for the whole world (`Id::Universal`).

use crate::terrain::cell::universal_bounds;
use crate::terrain::config::TerrainConfig;
use crate::terrain::presentation::TerrainPresentationAssets;
use crate::terrain::sdf::{ComposedTerrain, TerrainSdf};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex};
use lod::lod_ref::LodRef;

/// Shared heightfield noise used by every terrain cell and grading search.
#[derive(Debug, Clone, Component)]
pub struct BaseTerrainNoise {
	pub seed: u32,
	pub height_scale: f32,
	pub sdf: TerrainSdf,
}

impl BaseTerrainNoise {
	pub fn from_config(config: &TerrainConfig) -> Self {
		Self {
			seed: config.seed,
			height_scale: config.height_scale,
			sdf: TerrainSdf::new(config.seed, config.height_scale),
		}
	}

	pub fn height_at(&self, x: f32, z: f32) -> f32 {
		self.sdf.height_at_with_all_modulations(x, z)
	}

	pub fn composed(&self) -> ComposedTerrain {
		ComposedTerrain::from_terrain(self.sdf.clone())
	}
}

impl<S> GenerationScheme<S> for BaseTerrainNoise
where
	S: GeneratingSpatialIndex<TerrainPresentationAssets>,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		GeneratingSpatialIndex::<TerrainPresentationAssets>::get_or_generate(
			spatial_index,
			Id::Universal,
			lod_ref,
		)?;
		let assets =
			<S as SpatialIndex<TerrainPresentationAssets>>::get(spatial_index, Id::Universal)?;
		Some((Self::from_config(&assets.config), universal_bounds()))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
