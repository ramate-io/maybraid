//! Cellular urbanization for Richmond developments (forest parallel).
//!
//! An urbanization cell is 1600 m. Hopscotch picks a well-known
//! [`UrbanizationKind`]. [`UrbanizationKind::None`] short-circuits with empty
//! leaves (no guillotine). Other kinds run an adaptive guillotine (≈200–600 m
//! leaves) and Bucket-Throw an [`UrbanDevelopmentKind`] per leaf. Convert to
//! Richmond `DevelopmentKind` at the development-models boundary. World
//! bullseyes use [`DEVELOPMENT_PRESENT_RADIUS_M`] /
//! [`DEVELOPMENT_GENERATE_RADIUS_M`] (1 km / 3 km), mirroring forest grove rings.

mod extent;
mod generation;
mod guillotine;
pub(crate) mod hopscotch;
mod index;
mod kind;
mod richmond;

pub use extent::{UrbanizationExtent, DEFAULT_URBANIZATION_EXTENT_XZ};
pub use generation::{
	UrbanizationGenerateBullseye, UrbanizationLodChan, UrbanizationPresentBullseye,
	DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M,
};
pub use guillotine::{guillotine_partition, UrbanizationGuillotineParams};
pub use hopscotch::{select as hopscotch_select, HopscotchNode};
pub use index::UrbanizationIndex;
pub use kind::{UrbanDevelopmentKind, UrbanizationKind, UrbanizationRecipe, WeightedDevelopment};
pub use richmond::{
	richmond_hopscotch, select_cell, select_cell_as, select_kind, DevelopmentLeaf,
	SelectedUrbanization, DEFAULT_HOP_BUDGET,
};
