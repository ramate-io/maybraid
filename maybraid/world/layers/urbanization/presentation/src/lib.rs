//! [`UrbanizationPresentationPlugin`]: hosts and padded cells through the model.
//!
//! [`PaddedCells`] presents padded replacements for [`Urbanization<U>`] through
//! [`TerrainPresentationPlugin`](terrain_layer_presentation::TerrainPresentationPlugin).

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use lod::LodPresentGate;
use layer_stack::{install_lod_present_gate, subscribe_mode, GenerationMode};
use terrain_layer_model::TerrainModel;
use terrain_layer_presentation::TerrainPresenter;
use urbanization_layer_model::{Urbanization, UrbanizationGeneration};

/// Host spawn and padded-cell presentation for urbanization model `U`.
pub trait UrbanizationPresentation: UrbanizationGeneration {
	fn install_hosts(app: &mut App);

	fn install_padded_cells(app: &mut App);
}

/// Marker for host-presenter subscriptions on [`Urbanization<U>`].
pub struct UrbanizationHosts;

/// Presents padded replacements for [`Urbanization<U>`].
pub struct PaddedCells;

impl<U: UrbanizationPresentation> TerrainPresenter<Urbanization<U>> for PaddedCells
where
	Urbanization<U>: terrain_layer_model::TerrainModel,
{
	fn install(app: &mut App) {
		U::install_padded_cells(app);
	}
}

/// Presents the built developments of model `U` while `Mode` is subscribed.
pub struct UrbanizationPresentationPlugin<Mode, U>(PhantomData<fn() -> (Mode, U)>);

impl<Mode, U> Default for UrbanizationPresentationPlugin<Mode, U> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

pub struct UrbanizationPresentationCore<U>(PhantomData<fn() -> U>);

impl<U> Default for UrbanizationPresentationCore<U> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<U: UrbanizationPresentation> Plugin for UrbanizationPresentationCore<U>
where
	Urbanization<U>: terrain_layer_model::TerrainModel,
{
	fn build(&self, app: &mut App) {
		U::install_hosts(app);
		app.init_resource::<LodPresentGate<(Urbanization<U>, UrbanizationHosts)>>();
	}
}

impl<Mode: GenerationMode, U: UrbanizationPresentation> Plugin
	for UrbanizationPresentationPlugin<Mode, U>
where
	Urbanization<U>: terrain_layer_model::TerrainModel,
{
	fn build(&self, app: &mut App) {
		subscribe_mode::<(Urbanization<U>, UrbanizationHosts), Mode>(app);
		install_lod_present_gate::<(Urbanization<U>, UrbanizationHosts), (Urbanization<U>, UrbanizationHosts)>(
			app,
		);
		if !app.is_plugin_added::<UrbanizationPresentationCore<U>>() {
			app.add_plugins(UrbanizationPresentationCore::<U>::default());
		}
	}

	fn finish(&self, app: &mut App) {
		Urbanization::<U>::require_generation(app);
	}
}

#[cfg(test)]
mod tests;
