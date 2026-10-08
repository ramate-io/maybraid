//! Richmond developments: place building hosts (floors, shafts, roofs).
//!
//! Analogous to `chico-groves`: a development fits confines and emits flattened
//! hosts. Les Halles emits one urban stack; Shepherds Village scatters independent
//! buildings; Shepherds Commune lays the same buildings along a connecting grade;
//! Ring Fort wraps a courtyard curtain wall with corner keeps.
//!
//! These are the developments as kit assemblies. Layouts that read the ground
//! do so through a [`SiteGround`] probe and return the [`PadPlan`]s they stand
//! on; generating them over real terrain is Richmond's concern.

pub mod archetypes;
pub mod connected;
pub mod connectivity;
pub mod curtain_ring;
pub mod finish;
pub mod ground;
pub mod keep;
mod layout;
pub mod les_halles;
pub mod market;
pub mod pad;
pub mod placed;
pub mod plan;
pub mod ring_fort;
pub mod scatter;
pub mod shepherds;
mod shepherds_fit;
pub mod temple_sanctum;

pub use archetypes::{
	ApartmentMonotower, SingleHighrise, SingleHighriseFloorPlan, SingleHighrisePlan,
	SingleHighriseShaftSlot, SingleHighriseStorey, Skybridge, SkybridgeBazaar,
	SolitaryWizardsTower, SuburbanHomes, TempleComplex,
};
pub use connected::{ConnectedDevelopment, DevelopmentEdge};
pub use connectivity::{corridor_levels, ConnectivityCorridor, ConnectivityGraph};
pub use curtain_ring::CurtainRing;
pub use finish::{DevelopmentFinish, DevelopmentFinishRole};
pub use ground::SiteGround;
pub use keep::{CircularTower, Keep, RingFortKeep, TrazaloidTower};
pub use les_halles::{courtyard_well_side, MixedUseLesHallesDevelopment, MixedUseLesHallesHost};
pub use market::{
	OldCityMarket, OldCityMarketCorridor, OldCityMarketSite, OldCityMarketTerrace,
	OldCityMarketTier, MARKET_PLATFORM_HEIGHT,
};
pub use pad::{PadForm, PadParams, PadPlan, PAD_BERM, PAD_EDGE_EASE, PAD_ROUND};
pub use placed::{BuildingFootprint, PlacedBuilding};
pub use plan::{cell_salt, sample_confines_yaw, yaw_about_xz, yawed_plan_aabb_extent};
pub use ring_fort::{
	GalleryColonnade, GalleryTerrace, RingFort, RingFortHost, RingFortJoin, RingFortSite,
	RingFortTower,
};
pub use scatter::{bounds_intersect, ScatterCandidate, ScatterChoice, ScatterPlan, ScatterRecipe};
pub use shepherds::{
	ShepherdsBuilding, ShepherdsCommune, ShepherdsCommuneCorridor, ShepherdsCommuneSite,
	ShepherdsFinish, ShepherdsHouse, ShepherdsHut, ShepherdsVillage, ShepherdsVillageBuilding,
	HOUSE_MAX_FOOTPRINT, HOUSE_MIN_FOOTPRINT, HOUSE_STOREY_HEIGHT, HUT_HEIGHT, HUT_MAX_FOOTPRINT,
	HUT_MIN_FOOTPRINT,
};
pub use temple_sanctum::{TempleSanctum, TempleSanctumComponents};
