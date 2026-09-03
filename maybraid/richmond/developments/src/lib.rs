//! Richmond developments: place building hosts (floors, shafts, roofs).
//!
//! Analogous to `chico-groves`: a development fits confines and emits flattened
//! hosts. Les Halles emits one urban stack; Shepherds Village scatters independent
//! buildings; Shepherds Commune lays the same buildings along a connecting grade.

pub mod les_halles;
pub mod placed;
pub mod shepherds;

pub use les_halles::{courtyard_well_side, MixedUseLesHallesDevelopment, MixedUseLesHallesHost};
pub use placed::{BuildingFootprint, PlacedBuilding};
pub use shepherds::{
	ShepherdsBuilding, ShepherdsCommune, ShepherdsCommuneCorridor, ShepherdsFinish, ShepherdsHouse,
	ShepherdsHut, ShepherdsVillage, ShepherdsVillageBuilding, HOUSE_MAX_FOOTPRINT,
	HOUSE_MIN_FOOTPRINT, HOUSE_STOREY_HEIGHT, HUT_HEIGHT, HUT_MAX_FOOTPRINT, HUT_MIN_FOOTPRINT,
};
