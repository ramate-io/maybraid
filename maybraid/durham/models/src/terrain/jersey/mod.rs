//! Jersey landform stamps on per-family guillotine partitions.
//!
//! Each family owns its own controller grid (cell size + origin offset) and
//! guillotine seed, so leaf seams do not coincide across families. Stack per
//! family:
//!
//! `ControllerLayout` → `ControllerCell` (cuts) → leaf `Id` → `StampCell`
//!
//! There is no stored guillotine-identity layer: a valid leaf [`lod::gen::Id`]
//! down-levels to its cell. [`crate::terrain::Terrain`] pulls stamp cells
//! directly.
//!
//! Modules:
//! - [`configs`] — universal per-family cut + stamp params
//! - [`shared`] — offset grids, cut helpers, leaf discovery
//! - [`plateau`] / [`massif`] / [`canyon`] / [`pocket_water`] / [`rolling`] /
//!   [`valley`] — independent family stacks

pub mod canyon;
pub mod configs;
pub mod family_macro;
pub mod massif;
pub mod plateau;
pub mod pocket_water;
pub mod rolling;
pub mod shared;
pub mod valley;

#[cfg(test)]
mod tests;

pub use canyon::{
	original_ids_for_canyon_leaves, BootstrapCanyonControllerLayout, CanyonControllerCell,
	CanyonControllerLayout, CanyonStampCell,
};
pub use configs::{BootstrapJerseyStampConfigs, FamilyGuillotineConfig, JerseyStampConfigs};
pub use massif::{
	original_ids_for_massif_leaves, BootstrapMassifControllerLayout, MassifControllerCell,
	MassifControllerLayout, MassifStampCell,
};
pub use plateau::{
	original_ids_for_plateau_leaves, BootstrapPlateauControllerLayout, PlateauControllerCell,
	PlateauControllerLayout, PlateauStampCell,
};
pub use pocket_water::{
	original_ids_for_pocket_water_leaves, BootstrapPocketWaterControllerLayout,
	PocketWaterControllerCell, PocketWaterControllerLayout, PocketWaterStampCell,
};
pub use rolling::{
	original_ids_for_rolling_leaves, BootstrapRollingControllerLayout, RollingControllerCell,
	RollingControllerLayout, RollingStampCell,
};
pub use valley::{
	original_ids_for_valley_leaves, BootstrapValleyControllerLayout, ValleyControllerCell,
	ValleyControllerLayout, ValleyStampCell,
};
