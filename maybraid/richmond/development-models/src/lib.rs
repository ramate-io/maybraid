//! Richmond development models: 200 m lattice, pads, Les Halles, and Shepherds Village.
//!
//! Selection / pad / host generation on top of composed Durham [`Terrain`].
//! The crate plugin also installs SceneRef, urban surface MaterialRef, placeholder
//! wireframes, and the Richmond building LOD stack so playgrounds present
//! [`TerrainWithPads`] and building GLBs without assembling those plugins themselves.

pub mod buildings_lod;
pub mod cell;
pub mod config;
pub mod development;
pub mod finish;
pub mod generation;
pub mod hydro;
pub mod index;
pub mod les_halles;
pub mod pad;
pub mod padded;
pub mod plugin;
pub mod presentation;
pub mod shepherds;
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
pub use development::{select_kind, DevelopmentCell, DevelopmentKind, DevelopmentPad};
pub use finish::DevelopmentFinish;
pub use hydro::{composed_height_at, hydro_overlaps_xz, terrain_hydro_overlaps};
pub use index::{
	DevelopmentEntryStore, DevelopmentIndex, LesHallesStoreView, PaddedStoreView,
	ShepherdsVillageStoreView,
};
pub use les_halles::LesHallesDevelopment;
pub use pad::{cell_bounds2, PadComplex, PadNode, PadParams, PadPrimitive};
pub use padded::{PresentedPaddedTerrainScene, TerrainWithPads};
pub use plugin::{register_richmond_development_models_plugin, RichmondDevelopmentModelsPlugin};
pub use presentation::{PaddedTerrainPresenter, PaddedTerrainPresenterState};
pub use shepherds::ShepherdsVillageDevelopment;
