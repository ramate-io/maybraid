//! Jersey landform stamps on per-family dual-band guillotine partitions.
//!
//! Each family owns independent **low-pass** (detail) and **high-pass**
//! (regional) controller grids — different guillotine cell-size ranges,
//! controller roots, origin offsets, cut seeds, likelihoods, spatial
//! correlation lengths, and stamp **strength** ranges. Leaf seams therefore do
//! not coincide across families or bands.
//!
//! Horizontal footprint follows leaf/cell size (`*_frac`). Vertical amplitude
//! is driven by per-leaf [`jersey_terrain_stamps::StampStrength`] sampled from
//! each band's `strength: (min, max)`.
//!
//! Stack per band:
//!
//! `ControllerLayout` → `ControllerCell` (cuts) → leaf `Id` → `StampCell`
//!
//! There is no stored guillotine-identity layer: a valid leaf [`lod::gen::Id`]
//! down-levels to its cell. [`crate::terrain::Terrain`] pulls stamp cells
//! directly (high-pass first, then low-pass).
//!
//! Modules:
//! - [`configs`] — universal dual-band cut + stamp + likelihood params
//! - [`shared`] — offset grids, cut helpers, leaf discovery, occupancy
//! - [`plateau`] / [`massif`] / [`canyon`] / [`pocket_water`] / [`rolling`] /
//!   [`valley`] — independent family stacks

pub mod canyon;
pub mod configs;
pub mod family_macro;
pub mod layouts;
pub mod massif;
pub mod plateau;
pub mod pocket_water;
pub mod rolling;
pub mod shared;
pub mod valley;

#[cfg(test)]
mod tests;

pub use canyon::{
	original_ids_for_canyon_high_pass_leaves, original_ids_for_canyon_low_pass_leaves,
	BootstrapCanyonHighPassControllerLayout, BootstrapCanyonLowPassControllerLayout,
	CanyonHighPassControllerCell, CanyonHighPassControllerLayout, CanyonHighPassStampCell,
	CanyonLowPassControllerCell, CanyonLowPassControllerLayout, CanyonLowPassStampCell,
};
pub use configs::{
	BootstrapJerseyStampConfigs, DualBandFamilyConfig, FamilyGuillotineConfig, JerseyStampConfigs,
};
pub use layouts::JerseyControllerLayouts;
pub use massif::{
	original_ids_for_massif_high_pass_leaves, original_ids_for_massif_low_pass_leaves,
	BootstrapMassifHighPassControllerLayout, BootstrapMassifLowPassControllerLayout,
	MassifHighPassControllerCell, MassifHighPassControllerLayout, MassifHighPassStampCell,
	MassifLowPassControllerCell, MassifLowPassControllerLayout, MassifLowPassStampCell,
};
pub use plateau::{
	original_ids_for_plateau_high_pass_leaves, original_ids_for_plateau_low_pass_leaves,
	BootstrapPlateauHighPassControllerLayout, BootstrapPlateauLowPassControllerLayout,
	PlateauHighPassControllerCell, PlateauHighPassControllerLayout, PlateauHighPassStampCell,
	PlateauLowPassControllerCell, PlateauLowPassControllerLayout, PlateauLowPassStampCell,
};
/// Compatibility alias: low-pass plateau layout (detail band).
pub type PlateauControllerLayout = PlateauLowPassControllerLayout;
pub use pocket_water::{
	original_ids_for_pocket_water_high_pass_leaves, original_ids_for_pocket_water_low_pass_leaves,
	BootstrapPocketWaterHighPassControllerLayout, BootstrapPocketWaterLowPassControllerLayout,
	PocketWaterHighPassControllerCell, PocketWaterHighPassControllerLayout,
	PocketWaterHighPassStampCell, PocketWaterLowPassControllerCell,
	PocketWaterLowPassControllerLayout, PocketWaterLowPassStampCell,
};
pub use rolling::{
	original_ids_for_rolling_high_pass_leaves, original_ids_for_rolling_low_pass_leaves,
	BootstrapRollingHighPassControllerLayout, BootstrapRollingLowPassControllerLayout,
	RollingHighPassControllerCell, RollingHighPassControllerLayout, RollingHighPassStampCell,
	RollingLowPassControllerCell, RollingLowPassControllerLayout, RollingLowPassStampCell,
};
pub use valley::{
	original_ids_for_valley_high_pass_leaves, original_ids_for_valley_low_pass_leaves,
	BootstrapValleyHighPassControllerLayout, BootstrapValleyLowPassControllerLayout,
	ValleyHighPassControllerCell, ValleyHighPassControllerLayout, ValleyHighPassStampCell,
	ValleyLowPassControllerCell, ValleyLowPassControllerLayout, ValleyLowPassStampCell,
};
