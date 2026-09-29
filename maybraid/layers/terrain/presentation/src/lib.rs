//! [`TerrainPresentationPlugin`]: installs a presenter owned by a presentation crate.
//!
//! Model crates hold contracts and generation. If it draws, it lives here (or in
//! another presentation crate), keyed on a presenter type rather than the model.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use terrain_layer_model::TerrainModel;

/// Installs cell presentation for [`Self::Model`].
pub trait TerrainPresenter: Send + Sync + 'static {
	type Model: TerrainModel;

	fn install(app: &mut App);
}

/// Presents through `P`, e.g. `TerrainPresentationPlugin<DurhamCellPresenter>`.
pub struct TerrainPresentationPlugin<P>(PhantomData<fn() -> P>);

impl<P> Default for TerrainPresentationPlugin<P> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<P: TerrainPresenter> Plugin for TerrainPresentationPlugin<P> {
	fn build(&self, app: &mut App) {
		P::install(app);
	}

	fn finish(&self, app: &mut App) {
		<P::Model as TerrainModel>::require_generation(app);
	}
}

#[cfg(test)]
mod tests;
