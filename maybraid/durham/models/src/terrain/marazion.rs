//! Marazion pocket-water LOD stack ([RFC-127 §3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#31-marazion-pocket-water-stamping)).
//!
//! Dual-band: **low-pass** (leaf sides ≈200–600m) + **high-pass** (≈800m–3km).
//! Each band is `PrePocketLayout` → `PrePocketCell` → `PocketCell` → pocket-waters leaves.
//!
//! Author likelihood / cell size in [`low_pass`] / [`high_pass`] via
//! `define_marazion_band!` (same idea as Jersey `define_jersey_family!`).

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

pub use config::{BootstrapMarazionWatershedConfigs, MarazionBandConfig, MarazionWatershedConfigs};
pub use correction::{
	WatershedAproningCell, WatershedCarvingCell, HydroComplexCell,
	WatershedRimmingCell,
};
pub use high_pass::{
	bootstrap_pre_pocket_high_pass_layout, original_ids_for_marazion_pocket_waters_high_pass_leaves,
	original_ids_for_pocket_high_pass_cells, original_ids_for_pre_pocket_high_pass_cells,
	BootstrapPrePocketHighPassLayout, MarazionPocketWatersHighPass, PocketHighPassCell,
	PrePocketHighPassCell, PrePocketHighPassLayout,
};
pub use leaf_kind::{MarazionBandPass, MarazionLeafBounds, MarazionLeafKind};
pub use low_pass::{
	bootstrap_pre_pocket_low_pass_layout, original_ids_for_marazion_pocket_waters_low_pass_leaves,
	original_ids_for_pocket_low_pass_cells, original_ids_for_pre_pocket_low_pass_cells,
	BootstrapPrePocketLowPassLayout, MarazionPocketWatersLowPass, PocketLowPassCell,
	PrePocketLowPassCell, PrePocketLowPassLayout,
};
pub use pocket_water::MarazionPocketWater;

/// Convenience aliases (low-pass) for call sites that still expect singular names.
pub type PrePocketLayout = PrePocketLowPassLayout;
pub type PrePocketCell = PrePocketLowPassCell;
pub type PocketCell = PocketLowPassCell;
pub type MarazionPocketWaters = MarazionPocketWatersLowPass;
/// Historical alias — prefer [`MarazionPocketWatersLowPass`].
pub type MarazionLakeCell = MarazionPocketWatersLowPass;
/// Historical alias — prefer [`MarazionPocketWatersLowPass`].
pub type MarazionLakeLowPassCell = MarazionPocketWatersLowPass;
/// Historical alias — prefer [`MarazionPocketWatersHighPass`].
pub type MarazionLakeHighPassCell = MarazionPocketWatersHighPass;

pub use original_ids_for_marazion_pocket_waters_low_pass_leaves as original_ids_for_marazion_lake_leaves;
pub use original_ids_for_marazion_pocket_waters_low_pass_leaves as original_ids_for_marazion_lake_low_pass_leaves;
pub use original_ids_for_marazion_pocket_waters_high_pass_leaves as original_ids_for_marazion_lake_high_pass_leaves;
pub use original_ids_for_pocket_low_pass_cells as original_ids_for_pocket_cells;
pub use original_ids_for_pre_pocket_low_pass_cells as original_ids_for_pre_pocket_cells;
