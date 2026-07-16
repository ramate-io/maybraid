//! Universal base terrain noise — one value for the whole world (`Id::Universal`).

use crate::terrain::compose::TerrainConfig;
use crate::terrain::presentation::HasTerrainPresentationAssets;
use crate::terrain::sdf::{ComposedTerrain, TerrainSdf};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;
use lod::gen::{GenerationScheme, Id, OriginalId};
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

/// Large AABB covering typical playground extents; identity is [`Id::Universal`].
pub fn universal_bounds() -> Aabb3d {
	Aabb3d::from_min_max(Vec3::splat(-1_000_000.0), Vec3::splat(1_000_000.0))
}

impl<S> GenerationScheme<S> for BaseTerrainNoise
where
	S: HasTerrainPresentationAssets,
{
	fn original_ids_for(_spatial_index: &mut S, _region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId::universal()]
	}

	fn build_with_id(spatial_index: &mut S, id: Id, _lod_ref: &LodRef) -> Option<(Self, Aabb3d)> {
		if id != Id::Universal {
			return None;
		}
		let config = &spatial_index.presentation_assets().config;
		Some((Self::from_config(config), universal_bounds()))
	}

	fn descendants_with_lod(_id: Id, _spatial_index: &mut S, _lod_ref: &LodRef) {}
}
