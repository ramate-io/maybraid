//! Watershed pocket-water LOD stack ([RFC-127 §3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#31-marazion-pocket-water-stamping)).
//!
//! Dual-band: **low-pass** (leaf sides ≈200–600m) + **high-pass** (≈800m–3km).
//! Each band is `PrePocketLayout` → `PrePocketCell` → `PocketCell` → pocket-waters leaves.
//!
//! Author likelihood / cell size in [`low_pass`] / [`high_pass`] via
//! `define_marazion_band!` (same idea as Stamp `define_stamp_family!`).

pub mod band_macro;
pub mod bog;
pub mod config;
pub mod correction;
pub mod height;
pub mod high_pass;
pub mod lake;
pub mod leaf_kind;
pub mod low_pass;
pub mod pocket_water;
pub mod pre_pocket;
pub mod stream;

pub use config::{BootstrapWatershedConfigs, WatershedBandConfig, WatershedConfigs};
pub use correction::{
	HydroComplexCell, WatershedAproningCell, WatershedCarvingCell, WatershedRimmingCell,
};
pub use high_pass::{
	bootstrap_pre_pocket_high_pass_layout,
	original_ids_for_marazion_pocket_waters_high_pass_leaves,
	original_ids_for_pocket_high_pass_cells, original_ids_for_pre_pocket_high_pass_cells,
	BootstrapPrePocketHighPassLayout, PocketHighPassCell, PocketWatersHighPass,
	PrePocketHighPassCell, PrePocketHighPassLayout,
};
pub use leaf_kind::{WatershedBandPass, WatershedLeafBounds, WatershedLeafKind};
pub use low_pass::{
	bootstrap_pre_pocket_low_pass_layout, original_ids_for_marazion_pocket_waters_low_pass_leaves,
	original_ids_for_pocket_low_pass_cells, original_ids_for_pre_pocket_low_pass_cells,
	BootstrapPrePocketLowPassLayout, PocketLowPassCell, PocketWatersLowPass, PrePocketLowPassCell,
	PrePocketLowPassLayout,
};
pub use pocket_water::PocketWater;

/// Convenience aliases (low-pass) for call sites that still expect singular names.
pub type PrePocketLayout = PrePocketLowPassLayout;
pub type PrePocketCell = PrePocketLowPassCell;
pub type PocketCell = PocketLowPassCell;
pub type PocketWaters = PocketWatersLowPass;
pub use original_ids_for_marazion_pocket_waters_high_pass_leaves as original_ids_for_marazion_lake_high_pass_leaves;
pub use original_ids_for_marazion_pocket_waters_low_pass_leaves as original_ids_for_marazion_lake_leaves;
pub use original_ids_for_marazion_pocket_waters_low_pass_leaves as original_ids_for_marazion_lake_low_pass_leaves;
pub use original_ids_for_pocket_low_pass_cells as original_ids_for_pocket_cells;
pub use original_ids_for_pre_pocket_low_pass_cells as original_ids_for_pre_pocket_cells;
