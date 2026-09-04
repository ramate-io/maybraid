//! Richmond developments: place building hosts (floors, shafts, roofs).
//!
//! Analogous to `chico-groves`: a development fits confines and emits flattened
//! hosts. Les Halles emits one urban stack; Shepherds Village scatters independent
//! buildings; Shepherds Commune lays the same buildings along a connecting grade;
//! Ring Fort wraps a courtyard curtain wall with corner keeps.

pub mod connected;
pub mod curtain_ring;
pub mod keep;
pub mod les_halles;
pub mod placed;
pub mod ring_fort;
pub mod shepherds;

pub use connected::{ConnectedDevelopment, DevelopmentEdge};
pub use curtain_ring::CurtainRing;
pub use keep::{CircularTower, Keep, RingFortKeep, TrazaloidTower};
pub use les_halles::{courtyard_well_side, MixedUseLesHallesDevelopment, MixedUseLesHallesHost};
pub use placed::{BuildingFootprint, PlacedBuilding};
pub use ring_fort::{RingFort, RingFortHost, RingFortJoin, RingFortSite, RingFortTower};
pub use shepherds::{
	ShepherdsBuilding, ShepherdsCommune, ShepherdsCommuneCorridor, ShepherdsCommuneSite,
	ShepherdsFinish, ShepherdsHouse, ShepherdsHut, ShepherdsVillage, ShepherdsVillageBuilding,
	HOUSE_MAX_FOOTPRINT, HOUSE_MIN_FOOTPRINT, HOUSE_STOREY_HEIGHT, HUT_HEIGHT, HUT_MAX_FOOTPRINT,
	HUT_MIN_FOOTPRINT,
};
