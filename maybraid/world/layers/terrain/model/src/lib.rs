//! Layer contract for generated world surfaces.
//!
//! A terrain model is a zero-sized marker naming one stage of the world stack:
//! `Durham`, `OnTerrain<Durham>`, `Urbanization<OnTerrain<Durham>>`. Data stays
//! in each model's own resources. [`TerrainModel::Read`] is the read-only borrow
//! of those resources, and [`TerrainView`] is what systems take.
//!
//! [`layer_stack::Generate`] / [`layer_stack::Present`] are keyed by
//! [`OnTerrain<T>`]. Every layer checks the stack beneath it in
//! [`Plugin::finish`](bevy::app::Plugin::finish) through
//! [`TerrainModel::require_generation`] instead of adding other layers itself.

mod contract;
mod generation;
mod model;
mod on_terrain;
mod view;

pub use contract::{
	terrain_streaming, TerrainContract, TerrainExtent, TerrainExtentKind, TerrainLayerSystems,
	TerrainStreaming,
};
pub use generation::TerrainGeneration;
pub use model::{HeightField, TerrainCell, TerrainModel};
pub use on_terrain::OnTerrain;
pub use view::TerrainView;

#[cfg(test)]
mod tests;
