//! [`UrbanizationPresentationPlugin`]: building hosts, building LOD, walk colliders.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use urbanization_layer_model::UrbanModel;

/// Presents the built developments of urbanized model `G`.
pub struct UrbanizationPresentationPlugin<G>(PhantomData<fn() -> G>);

impl<G> Default for UrbanizationPresentationPlugin<G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<G: UrbanModel> Plugin for UrbanizationPresentationPlugin<G> {
	fn build(&self, _app: &mut App) {
		todo!(
			"UrbanizationPresentationPlugin: fill from \
			 maybraid/layers/urbanization/presentation/src/lib_sketch.rs"
		)
	}

	fn finish(&self, app: &mut App) {
		G::require_generation(app);
	}
}
