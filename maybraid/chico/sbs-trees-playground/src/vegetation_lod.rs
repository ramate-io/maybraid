//! Vegetation LOD refresh production (camera → region messages).
//!
//! Two passes on separate channels:
//! - [`VegetationBullseye`] — infrequent outer volume (50m / 500m)
//! - [`VegetationSpotlight`] — tight cube following position (20m)

use bevy::prelude::*;
use lod::{Bullseye, LodSceneRefreshRegionPlugin, Spotlight};

/// Channel marker for bullseye [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationBullseye;

/// Channel marker for spotlight [`lod::LodSceneRefreshRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationSpotlight;

/// Camera nodes → bullseye + spotlight vegetation region messages.
pub struct VegetationLodRefreshPlugin;

impl Plugin for VegetationLodRefreshPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(Bullseye {
			inner: 50.0,
			outer: 500.0,
		})
		.insert_resource(Spotlight { extent: 20.0 })
		.add_plugins((
			LodSceneRefreshRegionPlugin::<Bullseye, With<Camera>, VegetationBullseye>::default(),
			LodSceneRefreshRegionPlugin::<Spotlight, With<Camera>, VegetationSpotlight>::default(),
		));
	}
}
