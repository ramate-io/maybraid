//! Region production: [`LodNode`] drivers → [`LodSceneRefreshRegion`] messages.

mod bullseye;
mod produce;
mod spotlight;

pub use bullseye::Bullseye;
pub use produce::{
	arm_keep_if_empty, produce_lod_refresh_regions, LodRefreshRegions, LodRefreshRegionsError,
	LodRefreshRegionsStatus, LodSceneRefreshRegion, LodSceneRefreshRegionPlugin,
};
pub use spotlight::Spotlight;
