//! Cellular forests for Chico vegetation ([RFC-183 §3.5]).
//!
//! A forest cell is 1600 m. Hopscotch picks a well-known layering. Each layer
//! Bucket-Throws a grove or `None`. Selected groves tile the cell at 100 m
//! ([`DEFAULT_FOREST_GROVE_TILE_XZ`]). Ground cover is omitted. Missing groves
//! (Conifer Lower Massives) are dropped, not aliased. `ForestGroveBiases` stay
//! default — forests do not bias construction seeds.

mod assemble;
mod chico;
mod extent;
mod forest;
mod hopscotch;
mod kind;
mod layer;
pub mod layerings;

pub use assemble::{assemble, grow_tile, AssembledForest, ForestGroveTile};
pub use chico::{chico_hopscotch, select_cell, select_layering, DEFAULT_HOP_BUDGET};
pub use extent::{ForestExtent, DEFAULT_FOREST_EXTENT_XZ, DEFAULT_FOREST_GROVE_TILE_XZ};
pub use forest::ChicoForest;
pub use hopscotch::{select as hopscotch_select, HopscotchNode};
pub use kind::{ForestGroveKind, ForestLayering, LayeringKind, SelectedLayers, WeightedGrove};
pub use layer::{select_layers, throw_layer};
