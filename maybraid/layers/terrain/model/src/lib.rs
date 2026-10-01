//! Layer contract for generated world surfaces.
//!
//! A terrain model is a zero-sized marker naming one stage of the world stack:
//! `Durham`, `OnTerrain<Durham>`, `Urbanization<OnTerrain<Durham>>`. Data stays
//! in each model's own resources. [`TerrainModel::Read`] is the read-only borrow
//! of those resources, and [`TerrainView`] is what systems take.
//!
//! Generation plugins are typed by the model they read. Presentation plugins are
//! typed by a presenter in a presentation crate. Every layer plugin checks the
//! stack beneath it in [`Plugin::finish`](bevy::app::Plugin::finish) through
//! [`TerrainModel::require_generation`] instead of adding other layers itself.

mod generation;
mod generation_mode;
mod model;
mod on_terrain;
mod require;
mod subscription;
mod view;

pub use generation::{
	BaseTerrainGenerationCore, BaseTerrainGenerationPlugin, BaseTerrainModeConfig,
	BaseTerrainScheme, TerrainGeneration,
};
pub use generation_mode::{
	in_generation_mode, ActiveGenerationMode, GenerationMode, GenerationModePlugin,
	GenerationModeSystems,
};
pub use subscription::{mode_subscribed, subscribe_mode, ModeSubscribers, ModeSubscription};
pub use model::{HeightField, TerrainCell, TerrainModel};
pub use on_terrain::OnTerrain;
pub use require::RequireLayer;
pub use view::TerrainView;

#[cfg(test)]
mod tests;
