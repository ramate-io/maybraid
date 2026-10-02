//! [`TerrainPresentationPlugin`]: installs a presenter owned by a presentation crate.
//!
//! Model crates hold contracts and generation. If it draws, it lives here (or in
//! another presentation crate). The plugin is `TerrainPresentationPlugin<Mode, M, P>`:
//! the mode, the model, then how it is drawn.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use terrain_layer_model::{subscribe_mode, GenerationMode, TerrainModel};

/// Installs cell presentation for model `M`.
pub trait TerrainPresenter<M: TerrainModel>: Send + Sync + 'static {
	fn install(app: &mut App);
}

/// Presents model `M` through `P` while `Mode` is subscribed.
pub struct TerrainPresentationPlugin<Mode, M, P>(PhantomData<fn() -> (Mode, M, P)>);

impl<Mode, M, P> Default for TerrainPresentationPlugin<Mode, M, P> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

pub struct TerrainPresentationCore<M, P>(PhantomData<fn() -> (M, P)>);

impl<M, P> Default for TerrainPresentationCore<M, P> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<M: TerrainModel, P: TerrainPresenter<M>> Plugin for TerrainPresentationCore<M, P> {
	fn build(&self, app: &mut App) {
		P::install(app);
	}
}

impl<Mode: GenerationMode, M: TerrainModel, P: TerrainPresenter<M>> Plugin
	for TerrainPresentationPlugin<Mode, M, P>
{
	fn build(&self, app: &mut App) {
		subscribe_mode::<(M, P), Mode>(app);
		if !app.is_plugin_added::<TerrainPresentationCore<M, P>>() {
			app.add_plugins(TerrainPresentationCore::<M, P>::default());
		}
	}

	fn finish(&self, app: &mut App) {
		M::require_generation(app);
	}
}

#[cfg(test)]
mod tests;
