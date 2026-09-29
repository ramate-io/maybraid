//! [`UrbanizationGenerationPlugin`]: urbanization cells, pads, and padded cells over `M`.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use terrain_layer_model::TerrainModel;

use crate::model::Urbanization;

/// Generates urbanization over model `M` and makes `Urbanization<M>` available.
///
/// Reads `M` (pad heights sample the inner surface, never pads). Writes
/// urbanization cells, development cells, built developments, and padded cells.
pub struct UrbanizationGenerationPlugin<M>(PhantomData<fn() -> M>);

impl<M> Default for UrbanizationGenerationPlugin<M> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<M> Plugin for UrbanizationGenerationPlugin<M>
where
	M: TerrainModel,
	Urbanization<M>: TerrainModel,
{
	fn build(&self, _app: &mut App) {
		todo!(
			"UrbanizationGenerationPlugin: fill from \
			 maybraid/layers/urbanization/model/src/generation_sketch.rs"
		)
	}

	fn finish(&self, app: &mut App) {
		M::require_generation(app);
	}
}
