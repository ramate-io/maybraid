//! Cellular forests for Chico vegetation ([RFC-183 §3.5]).
//!
//! A forest cell is 1600 m. Hopscotch picks a well-known layering. Each layer
//! Bucket-Throws a grove or `None`. Generate queries [`ChicoGrove`] (100 m ×
//! layer); [`ChicoForest`] is select-only. Ground cover is omitted. Missing
//! groves (Conifer Lower Massives) are dropped, not aliased.
//! `ForestGroveBiases` stay default — forests do not bias construction seeds.

mod assemble;
mod blend;
mod chico;
mod extent;
mod forest;
mod generation;
mod grove;
pub(crate) mod hopscotch;
mod host;
mod index;
mod kind;
mod layer;
pub mod layerings;
mod recipe;

pub use assemble::{
	assemble, assemble_isolated, grow_tile, presenting_recipes, AssembledForest, ForestGroveTile,
	NeighborLayers,
};
pub use blend::{
	GROVE_BLEND_INFLUENCE, GROVE_BLEND_NOISE, GROVE_BLEND_RADIUS, GROVE_BLEND_TEMPERATURE,
};
pub use chico::{chico_hopscotch, select_cell, select_layering, DEFAULT_HOP_BUDGET};
pub use extent::{ForestExtent, DEFAULT_FOREST_EXTENT_XZ, DEFAULT_FOREST_GROVE_TILE_XZ};
pub use forest::{neighbor_layers, ChicoForest};
pub use generation::{
	ForestGenerateBullseye, ForestLodChan, ForestPresentBullseye, ForestPresentLattice,
	GROVE_GENERATE_RADIUS_M, GROVE_PRESENT_RADIUS_M,
};
pub use grove::{grove_from_id, grove_id, ChicoGrove};
pub use hopscotch::{select as hopscotch_select, HopscotchNode};
pub use host::ChicoGroveHost;
pub use index::{forest_world_sample, ForestIndex};
pub use kind::{
	ForestGroveKind, ForestLayer, ForestLayering, LayerDropOut, LayeringKind, SelectedLayers,
	WeightedGrove, TUFT_DROP_MIN_HEIGHT_M,
};
pub use layer::{select_layers, throw_layer};
pub use recipe::ForestGroveRecipe;
