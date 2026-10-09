//! Maputo: furnishing over a ground that exposes [`FurnitureSlots`].
//!
//! [`Maputo<G>`] bins world-space slots into 50 m cells and presents flattened
//! hosts around the camera. Vegetation and mobs do not wrap it.

mod cell;
mod colliders;
mod host;
mod index;
mod model;
mod present;
pub mod shared;
mod slots;
mod stream;

pub use cell::{
	FurnitureCellExtent, FURNITURE_CELL_SIZE, FURNITURE_GENERATE_RADIUS, FURNITURE_PRESENT_RADIUS,
};
pub use colliders::FurnitureWalkCollider;
pub use host::{spawn_furniture_cell, FurnitureCell};
pub use index::FurnitureIndex;
pub use model::Maputo;
pub use present::{FurnitureLodChan, FurnitureRefresh, PresentedFurnitureCellId};
pub use shared::{
	DevelopmentSlots, Furnished, FurnitureNeighborhood, MaputoNodes, MaputoPresentationPlugin,
};
pub use slots::{FurnishedDevelopment, FurnitureSlots};
pub use stream::install_furnishing_stream;

#[cfg(test)]
mod tests;
