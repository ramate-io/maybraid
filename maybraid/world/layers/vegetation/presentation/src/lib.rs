//! [`VegetationPresentationPlugin`]: grove hosts, bump-outs, and materials through the model.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use lod::LodPresentGate;
use layer_stack::{install_lod_present_gate, subscribe_mode, GenerationMode};
use terrain_layer_model::TerrainModel;
use vegetation_layer_model::{Vegetation, VegetationGeneration};

/// Grove hosts, bump-out overlays, and the material-library hookup for model `V`.
pub trait VegetationPresentation: VegetationGeneration {
	fn install_groves(app: &mut App);

	fn install_bump_outs(app: &mut App);

	fn install_materials(app: &mut App);
}

/// Marker for vegetation-presenter subscriptions on [`Vegetation<V>`].
pub struct VegetationPresent;

/// Presents generated vegetation of model `V` while `Mode` is subscribed.
pub struct VegetationPresentationPlugin<Mode, V>(PhantomData<fn() -> (Mode, V)>);

impl<Mode, V> Default for VegetationPresentationPlugin<Mode, V> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

pub struct VegetationPresentationCore<V>(PhantomData<fn() -> V>);

impl<V> Default for VegetationPresentationCore<V> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<V: VegetationPresentation> Plugin for VegetationPresentationCore<V>
where
	Vegetation<V>: terrain_layer_model::TerrainModel,
{
	fn build(&self, app: &mut App) {
		V::install_materials(app);
		V::install_groves(app);
		V::install_bump_outs(app);
		app.init_resource::<LodPresentGate<(Vegetation<V>, VegetationPresent)>>();
	}
}

impl<Mode: GenerationMode, V: VegetationPresentation> Plugin
	for VegetationPresentationPlugin<Mode, V>
where
	Vegetation<V>: terrain_layer_model::TerrainModel,
{
	fn build(&self, app: &mut App) {
		subscribe_mode::<(Vegetation<V>, VegetationPresent), Mode>(app);
		install_lod_present_gate::<(Vegetation<V>, VegetationPresent), (Vegetation<V>, VegetationPresent)>(
			app,
		);
		if !app.is_plugin_added::<VegetationPresentationCore<V>>() {
			app.add_plugins(VegetationPresentationCore::<V>::default());
		}
	}

	fn finish(&self, app: &mut App) {
		Vegetation::<V>::require_generation(app);
	}
}

#[cfg(test)]
mod tests;
