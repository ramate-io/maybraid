//! Marazion pocket-water LOD stack ([RFC-127 §3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#31-marazion-pocket-water-stamping)).
//!
//! Runs **after** [`crate::terrain::PreWatershedTerrain`] so plateau / rim / apron
//! lakes are last and wet fills are safe by construction.
//!
//! `PrePocketLayout` → `PrePocketCell` → `PocketCell` → `MarazionLakeCell`

pub mod config;
pub mod lake;
pub mod pocket;
pub mod pre_pocket;

pub use config::{BootstrapMarazionWatershedConfigs, MarazionWatershedConfigs};
pub use lake::{original_ids_for_marazion_lake_leaves, MarazionLakeCell};
pub use pocket::{original_ids_for_pocket_cells, PocketCell};
pub use pre_pocket::{
	original_ids_for_pre_pocket_cells, pre_pocket_layout_from_configs, BootstrapPrePocketLayout,
	PrePocketCell, PrePocketLayout,
};
