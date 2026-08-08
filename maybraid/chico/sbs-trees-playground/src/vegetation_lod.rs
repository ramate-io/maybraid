//! Vegetation LOD refresh production (camera → region outlet).

use bevy::prelude::*;
use lod::{InnerOuterLattice, LodRefreshProductionPlugin};

/// Outlet marker for vegetation [`lod::LodSceneRefreshRegions`].
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct Vegetation;

/// Camera nodes (`LodViewer` + Bevy [`Camera`]) → [`Vegetation`] region outlet.
pub struct VegetationLodRefreshPlugin;

impl Plugin for VegetationLodRefreshPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(InnerOuterLattice {
			cell_size: 100.0,
			ring_radius: 10,
		})
		.add_plugins(LodRefreshProductionPlugin::<InnerOuterLattice, With<Camera>, Vegetation>::default());
	}
}
