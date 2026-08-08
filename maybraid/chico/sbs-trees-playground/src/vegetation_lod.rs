//! Vegetation LOD refresh production (camera → region messages).

use bevy::prelude::*;
use lod::{InnerOuterLattice, LodSceneRefreshRegionPlugin};

/// Channel marker for vegetation [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct Vegetation;

/// Camera nodes → [`Vegetation`] region messages via [`InnerOuterLattice`].
pub struct VegetationLodRefreshPlugin;

impl Plugin for VegetationLodRefreshPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(InnerOuterLattice {
			cell_size: 100.0,
			ring_radius: 10,
		})
		.add_plugins(
			LodSceneRefreshRegionPlugin::<InnerOuterLattice, With<Camera>, Vegetation>::default(),
		);
	}
}
