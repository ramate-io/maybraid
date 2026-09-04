//! Richmond development models: 300 m lattice, terrain pads, and a unified
//! generation path for solitary buildings, campuses, and neighborhoods.
//!
//! Selection / pad / host generation on top of composed Durham [`Terrain`].
//! The crate plugin also installs SceneRef, urban surface MaterialRef, placeholder
//! wireframes, and the Richmond building LOD stack so playgrounds present
//! [`TerrainWithPads`] and building GLBs without assembling those plugins themselves.

mod archetype_generation;
pub mod artifact;
pub mod buildings_lod;
pub mod cell;
pub mod commune;
pub mod config;
pub mod connectivity;
pub mod development;
pub mod finish;
pub mod generation;
pub mod host;
pub mod hydro;
pub mod index;
pub mod les_halles;
pub mod market;
pub mod pad;
pub mod padded;
pub mod plugin;
pub mod presentation;
pub mod ring_fort;
pub mod scatter;
pub mod shepherds;
mod shepherds_fit;
pub mod village;

pub use archetype_generation::PlacedDevelopment;
pub use artifact::BuiltDevelopment;
pub use buildings_lod::{
	register_developments_buildings_lod_plugin, BuildingsBullseye, BuildingsCull,
	BuildingsSpotlight, DevelopmentsBuildingsLodPlugin,
};
pub use cell::{
	cell_selected, yaw_about_xz, DevelopmentExtent, BUILDING_INSET, DEFAULT_LIKELIHOOD,
	DEFAULT_SPATIAL_CORRELATION, DEVELOPMENT_CELL_SIZE, LES_HALLES_MAX_FOOTPRINT, PAD_BERM,
	PAD_EDGE_EASE, PAD_ROUND, RING_FORT_MAX_FOOTPRINT, RING_FORT_MIN_FOOTPRINT,
};
pub use config::DevelopmentConfig;
pub use development::{
	select_kind, ArchetypeCell, DevelopmentCell, DevelopmentContent, DevelopmentKind,
	DevelopmentPad, LesHallesCell, OldCityMarketCell, RingFortCell, ShepherdsCommuneCell,
	ShepherdsVillageCell,
};
pub use finish::{DevelopmentFinish, DevelopmentFinishRole};
pub use host::{DevelopmentHost, DevelopmentHosts};
pub use hydro::{
	composed_height_at, composed_height_upper_on_rect, hydro_overlaps_xz, terrain_hydro_overlaps,
};
pub use index::{
	BuiltDevelopmentStoreView, DevelopmentCellStoreView, DevelopmentEntryStore, DevelopmentIndex,
	PaddedStoreView,
};
pub use les_halles::LesHallesDevelopment;
pub use pad::{
	cell_bounds2, nodes_from_graded_polyline, PadComplex, PadNode, PadParams, PadPrimitive,
	PlacedBuildingPad,
};
pub use padded::{PresentedPaddedTerrainScene, TerrainWithPads};
pub use plugin::{register_richmond_development_models_plugin, RichmondDevelopmentModelsPlugin};
pub use presentation::{PaddedTerrainPresenter, PaddedTerrainPresenterState};
pub use ring_fort::RingFortDevelopment;
pub use scatter::{bounds_intersect, ScatterCandidate, ScatterChoice, ScatterPlan, ScatterRecipe};
pub use shepherds::{ShepherdsCommuneDevelopment, ShepherdsVillageDevelopment};
