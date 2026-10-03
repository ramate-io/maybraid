//! [`FurnishingPresentationPlugin`]: cells, shaders, and kit meshes through the model.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use furnishing_layer_model::{Furnishing, FurnishingGeneration};
use layer_stack::{install_lod_present_gate, subscribe_mode, GenerationMode};

/// Spawn a cell, tag it, and refresh flattened hosts for model `F`.
pub trait FurnishingPresentation: FurnishingGeneration {
	/// Channel whose [`lod::LodPresentGate`] this layer opens and closes.
	type Channel: Send + Sync + 'static;

	fn install_presentation(app: &mut App);
}

/// Marker for furnishing-presenter subscriptions on [`Furnishing<F>`].
pub struct FurnishingPresent;

/// Presents generated furniture of model `F` while `Mode` is subscribed.
pub struct FurnishingPresentationPlugin<Mode, F>(PhantomData<fn() -> (Mode, F)>);

impl<Mode, F> Default for FurnishingPresentationPlugin<Mode, F> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

pub struct FurnishingPresentationCore<F>(PhantomData<fn() -> F>);

impl<F> Default for FurnishingPresentationCore<F> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<F: FurnishingPresentation> Plugin for FurnishingPresentationCore<F> {
	fn build(&self, app: &mut App) {
		F::install_presentation(app);
	}
}

impl<Mode, F> Plugin for FurnishingPresentationPlugin<Mode, F>
where
	Mode: GenerationMode,
	F: FurnishingPresentation,
{
	fn build(&self, app: &mut App) {
		subscribe_mode::<(Furnishing<F>, FurnishingPresent), Mode>(app);
		install_lod_present_gate::<(Furnishing<F>, FurnishingPresent), F::Channel>(app);
		if !app.is_plugin_added::<FurnishingPresentationCore<F>>() {
			app.add_plugins(FurnishingPresentationCore::<F>::default());
		}
	}

	fn finish(&self, app: &mut App) {
		F::require_generation(app);
	}
}

#[cfg(test)]
mod tests;
