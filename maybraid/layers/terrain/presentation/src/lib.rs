//! [`TerrainPresentationPlugin`]: installs a presenter owned by a presentation crate.
//!
//! Model crates hold contracts and generation. If it draws, it lives here (or in
//! another presentation crate). The plugin is `TerrainPresentationPlugin<M, P>`:
//! the model first, then how it is drawn.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use terrain_layer_model::TerrainModel;

/// Installs cell presentation for model `M`.
pub trait TerrainPresenter<M: TerrainModel>: Send + Sync + 'static {
	fn install(app: &mut App);
}

/// Presents model `M` through `P`, e.g.
/// `TerrainPresentationPlugin<OnTerrain<Durham>, DurhamCells>`.
pub struct TerrainPresentationPlugin<M, P>(PhantomData<fn() -> (M, P)>);

impl<M, P> Default for TerrainPresentationPlugin<M, P> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<M: TerrainModel, P: TerrainPresenter<M>> Plugin for TerrainPresentationPlugin<M, P> {
	fn build(&self, app: &mut App) {
		P::install(app);
	}

	fn finish(&self, app: &mut App) {
		M::require_generation(app);
	}
}

#[cfg(test)]
mod tests;
