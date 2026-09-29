//! [`TerrainPresentationPlugin`]: fills, materials, and walk colliders for a model's cells.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use terrain_layer_model::TerrainModel;

/// Presents the stored cells of model `M`, e.g. raw Durham for
/// `OnTerrain<Durham>` or padded cells for `Urbanization<OnTerrain<Durham>>`.
pub struct TerrainPresentationPlugin<M>(PhantomData<fn() -> M>);

impl<M> Default for TerrainPresentationPlugin<M> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<M: TerrainModel> Plugin for TerrainPresentationPlugin<M> {
	fn build(&self, _app: &mut App) {
		todo!("TerrainPresentationPlugin: fill from maybraid/layers/terrain/presentation/src/lib_sketch.rs")
	}

	fn finish(&self, app: &mut App) {
		M::require_generation(app);
	}
}
