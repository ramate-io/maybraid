//! Cellular forests for Chico vegetation ([RFC-183 §3.5]).
//!
//! A forest cell is 1600 m. Hopscotch picks a well-known layering. Each layer
//! Bucket-Throws a grove or `None`. Selected groves tile the cell at 100 m
//! ([`DEFAULT_FOREST_GROVE_TILE_XZ`]). Ground cover is omitted. Missing groves
//! (Conifer Lower Massives) are dropped, not aliased. `ForestGroveBiases` stay
//! default — forests do not bias construction seeds.

mod assemble;
mod blend;
mod chico;
mod extent;
mod forest;
mod generation;
pub(crate) mod hopscotch;
mod index;
mod kind;
mod layer;
pub mod layerings;
mod recipe;

pub use assemble::{
	assemble, assemble_isolated, grow_tile, AssembledForest, ForestGroveTile, NeighborLayers,
};
pub use blend::{
	GROVE_BLEND_INFLUENCE, GROVE_BLEND_NOISE, GROVE_BLEND_RADIUS, GROVE_BLEND_TEMPERATURE,
};
pub use chico::{chico_hopscotch, select_cell, select_layering, DEFAULT_HOP_BUDGET};
pub use extent::{ForestExtent, DEFAULT_FOREST_EXTENT_XZ, DEFAULT_FOREST_GROVE_TILE_XZ};
pub use forest::{neighbor_layers, ChicoForest};
pub use generation::{
	ForestGenerateBullseye, ForestLodChan, ForestPresentBullseye, ForestPresentLattice,
};
pub use hopscotch::{select as hopscotch_select, HopscotchNode};
pub use index::ForestIndex;
pub use kind::{ForestGroveKind, ForestLayering, LayeringKind, SelectedLayers, WeightedGrove};
pub use layer::{select_layers, throw_layer};
