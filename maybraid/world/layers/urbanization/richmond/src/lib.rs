//! Richmond development models: urbanization leaves (default) or a legacy
//! 300 m occupancy lattice, terrain pads, and a unified generation path for
//! single buildings, campuses, and neighborhoods.
//!
//! Every node is a [`lod::gen::GenerationScheme`] over [`lod::hcsg::HcsgStorage`],
//! generic over a [`RichmondGround`]: a [`DevelopmentSite`] picks a kind, a
//! [`RichmondDevelopment`] fits it to the ground, [`Built`] fits its hosts, and
//! [`PaddedTerrain`] composes its pads into the ground's cells.
//! Cell discovery defaults to [`urbanization_cells`] guillotine leaves;
//! set [`DevelopmentConfig::sites`] to [`DevelopmentSites::Lattice`] for the dense
//! 300 m lattice. The crate plugin also installs SceneRef, urban surface MaterialRef,
//! placeholder wireframes, the Richmond building LOD stack, and Fixed-layer walk
//! colliders so playgrounds present [`TerrainWithPads`] and building GLBs without
//! assembling those plugins themselves.

mod archetype_generation;
pub mod artifact;
pub mod buildings_lod;
pub mod built;
pub mod cell;
pub mod compose;
pub mod config;
pub mod connectivity;
pub mod developments;
pub mod finish;
pub mod ground;
pub mod host;
pub mod layer;
pub mod layer_config;
pub mod layer_present;
pub mod layer_stream;
pub mod pad;
pub mod padded;
pub mod place;
pub mod plugin;
pub mod presentation;
pub mod scatter;
mod shepherds_fit;
pub mod storage;

pub use artifact::BuiltDevelopment;
pub use buildings_lod::{
	register_developments_buildings_lod_plugin, BuildingsBullseye, BuildingsCull,
	BuildingsSpotlight, DevelopmentsBuildingsLodPlugin,
};
pub use built::Built;
pub use cell::{
	cell_selected, yaw_about_xz, DevelopmentExtent, BUILDING_INSET, DEFAULT_LIKELIHOOD,
	DEFAULT_SPATIAL_CORRELATION, DEVELOPMENT_CELL_SIZE, LES_HALLES_MAX_FOOTPRINT, PAD_BERM,
	PAD_EDGE_EASE, PAD_ROUND, RING_FORT_MAX_FOOTPRINT, RING_FORT_MIN_FOOTPRINT,
};
pub use compose::PadComposable;
pub use config::{DevelopmentConfig, DevelopmentSites};
pub use developments::site::{
	select_kind, AuthoredCourtyard, AuthoredDevelopment, AuthoredDevelopments, DevelopmentKind,
	DevelopmentSite,
};
pub use developments::terrace::{TerraceCell, TerraceEnvelope, TerraceKind, TerracePlan};
pub use developments::{DevelopmentPad, RichmondDevelopment};
pub use finish::{DevelopmentFinish, DevelopmentFinishRole};
pub use ground::{hydro_overlaps_xz, GroundCell, GroundSampler, RichmondGround, SiteGround};
pub use host::{DevelopmentHost, DevelopmentHosts};
pub use layer::Richmond;
pub use layer_config::{
	DevelopmentFocus, RichmondConfig, UrbanizationStreamSpec, DEFAULT_URBANIZATION_NOISE,
	DEFAULT_URBANIZATION_STREAM_RADIUS, PLAYGROUND_LIKELIHOOD,
};
pub use layer_present::{
	present_richmond_hosts, spawn_development_hosts, spawn_tagged_host_entities,
	sync_raw_terrain_replacements, DevelopmentHostRoot, UrbanizationPaddedTerrainState,
	UrbanizationPresenterState,
};
pub use layer_stream::{
	install_urbanization_stream, parse_urbanization_kind, stream_radii_m, DevelopmentWindow,
	HostWindow,
};
pub use pad::{
	cell_bounds2, nodes_from_graded_polyline, PadComplex, PadNode, PadParams, PadPrimitive,
	PlacedBuildingPad,
};
pub use padded::{PaddedTerrain, PresentedPaddedTerrainScene, TerrainWithPads};
pub use place::{DiscoverablePlace, DiscoverablePlaceLabel, InteriorArea};
pub use plugin::{register_richmond_plugin, RichmondDevelopmentModelsPlugin};
pub use presentation::{PaddedTerrainPresenter, PaddedTerrainPresenterState};
pub use scatter::{bounds_intersect, ScatterCandidate, ScatterChoice, ScatterPlan, ScatterRecipe};
pub use storage::{column_bounds, register_richmond_nodes, RichmondNodes};

#[cfg(test)]
mod layer_tests;
