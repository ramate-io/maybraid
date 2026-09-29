//! [`VegetationPresentationPlugin`]: grow groves and bump-outs on model `G`.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use terrain_layer_model::{RequireLayer, TerrainModel};
use vegetation_layer_model::VegetationGenerationPlugin;

/// Presents generated vegetation on ground `G`. Pads reach groves through
/// `G`'s height (`Urbanization<…>` already composes them), not a special case.
pub struct VegetationPresentationPlugin<G>(PhantomData<fn() -> G>);

impl<G> Default for VegetationPresentationPlugin<G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<G: TerrainModel> Plugin for VegetationPresentationPlugin<G> {
	fn build(&self, _app: &mut App) {
		todo!(
			"VegetationPresentationPlugin: fill from \
			 maybraid/layers/vegetation/presentation/src/lib_sketch.rs"
		)
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
		app.require_layer::<VegetationGenerationPlugin, Self>();
	}
}
