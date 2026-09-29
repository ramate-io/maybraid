//! [`MobGenerationPlugin`]: which mob groups exist where, over urbanized ground `G`.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use terrain_layer_model::RequireLayer;
use urbanization_layer_model::UrbanModel;
use vegetation_layer_model::VegetationGenerationPlugin;

/// Mob cells, group kinds, and spawn anchors. Reads urbanization through
/// [`UrbanModel`] and forest layering from vegetation generation.
pub struct MobGenerationPlugin<G>(PhantomData<fn() -> G>);

impl<G> Default for MobGenerationPlugin<G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<G: UrbanModel> Plugin for MobGenerationPlugin<G> {
	fn build(&self, _app: &mut App) {
		todo!("MobGenerationPlugin: fill from maybraid/layers/mobs/model/src/lib_sketch.rs")
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
		app.require_layer::<VegetationGenerationPlugin, Self>();
	}
}
