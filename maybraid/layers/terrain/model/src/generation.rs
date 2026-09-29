//! Base of the stack: models that own a generation pipeline.

use bevy::app::{App, Plugin};

use crate::model::TerrainModel;

/// A model with its own generation stack (the bottom of the wiring diagram).
///
/// Wrappers such as `Urbanization<M>` are not `TerrainGeneration`; their
/// generation plugins are typed by the model they read instead.
pub trait TerrainGeneration: TerrainModel {
	type Config: Clone + Send + Sync + 'static;

	/// Register the model's stores, layout, and generate systems. No presentation.
	fn install_generation(app: &mut App, config: &Self::Config);
}

/// Generation for base model `T`, e.g. `BaseTerrainGenerationPlugin<Durham>`.
pub struct BaseTerrainGenerationPlugin<T: TerrainGeneration> {
	pub config: T::Config,
}

impl<T: TerrainGeneration> BaseTerrainGenerationPlugin<T> {
	pub fn new(config: T::Config) -> Self {
		Self { config }
	}
}

impl<T: TerrainGeneration> Plugin for BaseTerrainGenerationPlugin<T> {
	fn build(&self, app: &mut App) {
		T::install_generation(app, &self.config);
	}
}
