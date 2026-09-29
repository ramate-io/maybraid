//! [`MobPresentationPlugin`]: mob hosts grounded on `G`, surface fit, High LOD.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use mob_layer_model::MobGenerationPlugin;
use terrain_layer_model::RequireLayer;
use urbanization_layer_model::UrbanModel;

/// Presents generated mob groups on ground `G`.
///
/// Bound on [`UrbanModel`] only because mob generation needs urbanization today.
/// Presentation itself reads nothing but `G`'s height.
pub struct MobPresentationPlugin<G>(PhantomData<fn() -> G>);

impl<G> Default for MobPresentationPlugin<G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<G: UrbanModel> Plugin for MobPresentationPlugin<G> {
	fn build(&self, _app: &mut App) {
		todo!(
			"MobPresentationPlugin: fill from maybraid/layers/mobs/presentation/src/lib_sketch.rs"
		)
	}

	fn finish(&self, app: &mut App) {
		app.require_layer::<MobGenerationPlugin<G>, Self>();
	}
}
