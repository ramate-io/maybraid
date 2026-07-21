//! Marazion pocket-water LOD stack ([RFC-127 §3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#31-marazion-pocket-water-stamping)).
//!
//! Dual-band: **low-pass** (leaf sides ≈200–600m) + **high-pass** (≈800m–3km).
//! Each band is `PrePocketLayout` → `PrePocketCell` → `PocketCell` → `MarazionLakeCell`.

#[macro_use]
pub mod band_macro;
pub mod config;
pub mod lake;
pub mod pre_pocket;

pub use config::{
	BootstrapMarazionWatershedConfigs, MarazionBandConfig, MarazionWatershedConfigs,
};

define_marazion_band! {
	layout: PrePocketLowPassLayout,
	bootstrap_layout: BootstrapPrePocketLowPassLayout / bootstrap_pre_pocket_low_pass_layout,
	pre_cell: PrePocketLowPassCell,
	pocket: PocketLowPassCell,
	lake: MarazionLakeLowPassCell,
	pre_ids: original_ids_for_pre_pocket_low_pass_cells,
	pocket_ids: original_ids_for_pocket_low_pass_cells,
	lake_ids: original_ids_for_marazion_lake_low_pass_leaves,
	band_field: low_pass,
	default_fn: low_pass_default,
}

define_marazion_band! {
	layout: PrePocketHighPassLayout,
	bootstrap_layout: BootstrapPrePocketHighPassLayout / bootstrap_pre_pocket_high_pass_layout,
	pre_cell: PrePocketHighPassCell,
	pocket: PocketHighPassCell,
	lake: MarazionLakeHighPassCell,
	pre_ids: original_ids_for_pre_pocket_high_pass_cells,
	pocket_ids: original_ids_for_pocket_high_pass_cells,
	lake_ids: original_ids_for_marazion_lake_high_pass_leaves,
	band_field: high_pass,
	default_fn: high_pass_default,
}

/// Convenience aliases (low-pass) for call sites that still expect singular names.
pub type PrePocketLayout = PrePocketLowPassLayout;
pub type PrePocketCell = PrePocketLowPassCell;
pub type PocketCell = PocketLowPassCell;
pub type MarazionLakeCell = MarazionLakeLowPassCell;

pub use original_ids_for_marazion_lake_low_pass_leaves as original_ids_for_marazion_lake_leaves;
pub use original_ids_for_pocket_low_pass_cells as original_ids_for_pocket_cells;
pub use original_ids_for_pre_pocket_low_pass_cells as original_ids_for_pre_pocket_cells;
