//! Richmond development models: 200 m lattice, pads, Les Halles, Shepherds Village, and Shepherds Commune.
//!
//! Selection / pad / host generation on top of composed Durham [`Terrain`].
//! The crate plugin also installs SceneRef, urban surface MaterialRef, placeholder
//! wireframes, and the Richmond building LOD stack so playgrounds present
//! [`TerrainWithPads`] and building GLBs without assembling those plugins themselves.

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
pub mod pad;
pub mod padded;
pub mod plugin;
pub mod presentation;
pub mod scatter;
pub mod shepherds;
mod shepherds_fit;
pub mod village;

pub use buildings_lod::{
	register_developments_buildings_lod_plugin, BuildingsBullseye, BuildingsCull,
	BuildingsSpotlight, DevelopmentsBuildingsLodPlugin,
};
pub use cell::{
	cell_selected, yaw_about_xz, DevelopmentExtent, BUILDING_INSET, DEFAULT_LIKELIHOOD,
	DEFAULT_SPATIAL_CORRELATION, DEVELOPMENT_CELL_SIZE, LES_HALLES_MAX_FOOTPRINT, PAD_BERM,
	PAD_EDGE_EASE, PAD_ROUND,
};
pub use config::DevelopmentConfig;
pub use development::{
	select_kind, DevelopmentCell, DevelopmentContent, DevelopmentKind, DevelopmentPad,
	LesHallesCell, ShepherdsCommuneCell, ShepherdsVillageCell,
};
pub use finish::DevelopmentFinish;
pub use host::{DevelopmentHost, DevelopmentHosts};
pub use hydro::{
	composed_height_at, composed_height_upper_on_rect, hydro_overlaps_xz, terrain_hydro_overlaps,
};
pub use index::{
	DevelopmentCellStoreView, DevelopmentEntryStore, DevelopmentIndex, LesHallesStoreView,
	PaddedStoreView, ShepherdsCommuneStoreView, ShepherdsVillageStoreView,
};
pub use les_halles::LesHallesDevelopment;
pub use pad::{
	cell_bounds2, nodes_from_graded_polyline, PadComplex, PadNode, PadParams, PadPrimitive,
	PlacedBuildingPad,
};
pub use padded::{PresentedPaddedTerrainScene, TerrainWithPads};
pub use plugin::{register_richmond_development_models_plugin, RichmondDevelopmentModelsPlugin};
pub use presentation::{PaddedTerrainPresenter, PaddedTerrainPresenterState};
pub use scatter::{bounds_intersect, ScatterCandidate, ScatterChoice, ScatterPlan, ScatterRecipe};
pub use shepherds::{ShepherdsCommuneDevelopment, ShepherdsVillageDevelopment};
