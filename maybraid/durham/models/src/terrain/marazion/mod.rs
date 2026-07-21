//! Marazion lake LOD stack ([RFC-127 §3.1.3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3131-lake)).
//!
//! Runs **after** [`crate::terrain::PreWatershedTerrain`] so plateau / rim / apron
//! lakes are last and wet fills are safe by construction.

pub mod config;
pub mod lake;

pub use config::{
	BootstrapMarazionWatershedConfigs, MarazionWatershedConfigs, DEFAULT_LAKE_LEAF_SIZE,
};
pub use lake::{
	lake_layout_from_configs, original_ids_for_marazion_lake_leaves, BootstrapMarazionLakeLayout,
	MarazionLakeCell, MarazionLakeLayout,
};
