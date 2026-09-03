//! Richmond developments: place building hosts (floors, shafts, roofs).
//!
//! Analogous to `chico-groves`: a development fits confines and emits flattened
//! hosts. Les Halles emits one urban stack; Shepherds Village emits several
//! independently posed complete-building hosts.

pub mod les_halles;
pub mod placed;
pub mod shepherds;

pub use les_halles::{courtyard_well_side, MixedUseLesHallesDevelopment, MixedUseLesHallesHost};
pub use placed::{BuildingFootprint, PlacedBuilding};
pub use shepherds::{
	ShepherdsBuilding, ShepherdsFinish, ShepherdsHouse, ShepherdsHut, ShepherdsVillage,
	ShepherdsVillageBuilding, HOUSE_MAX_FOOTPRINT, HOUSE_MIN_FOOTPRINT, HOUSE_STOREY_HEIGHT,
	HUT_HEIGHT, HUT_MAX_FOOTPRINT, HUT_MIN_FOOTPRINT,
};
