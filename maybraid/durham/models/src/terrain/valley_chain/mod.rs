//! Jersey Valley Chain via guillotine-partitioned controller cells.
//!
//! Hybrid of [RFC-105](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain)
//! stamp construction and [RFC-127](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds)
//! pocket hierarchy (controller → guillotine leaves → stamp), scoped to valley
//! trains only — not the full Marazion pocket-water type set.
//!
//! Layers:
//! - [`config`] — universal [`JerseyValleyChainLayerConfig`]
//! - [`layout`] — controller grid
//! - [`controller`] — uniform cells that own [`comproc::guillotine::GuillotineCuts`]
//! - [`guillotine_cell`] — irregular leaf identities in the spatial index
//! - [`stamp`] — [`jersey_terrain_stamps::ValleyTrain`] on each leaf
//!
//! [`crate::terrain::Terrain`] pulls stamp cells directly (no compose
//! `GenerationScheme`).

pub mod config;
pub mod controller;
pub mod guillotine_cell;
pub mod layout;
pub mod stamp;

#[cfg(test)]
mod tests;

pub use config::{BootstrapJerseyValleyChainLayerConfig, JerseyValleyChainLayerConfig};
pub use controller::JerseyValleyChainControllerCell;
pub use guillotine_cell::{
	original_ids_for_guillotine_leaves, JerseyValleyChainGuillotineCell,
};
pub use layout::{
	BootstrapJerseyValleyChainControllerLayout, JerseyValleyChainControllerLayout,
	VALLEY_CHAIN_CONTROLLER_CELL_SIZE,
};
pub use stamp::JerseyValleyChainStampCell;
