//! [`TerrainPresentationPlugin`]: installs a model's own cell presentation.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use terrain_layer_model::TerrainPresentation;

/// Presents model `M` by calling [`TerrainPresentation::install_presentation`].
///
/// Raw Durham is `TerrainPresentationPlugin<OnTerrain<Durham>>`. Padded cells
/// (`Urbanization<_>`) are a later impl of the same hook.
pub struct TerrainPresentationPlugin<M>(PhantomData<fn() -> M>);

impl<M> Default for TerrainPresentationPlugin<M> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<M: TerrainPresentation> Plugin for TerrainPresentationPlugin<M> {
	fn build(&self, app: &mut App) {
		M::install_presentation(app);
	}

	fn finish(&self, app: &mut App) {
		M::require_generation(app);
	}
}
