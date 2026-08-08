//! Region production: [`LodNode`] drivers → [`LodSceneRefreshRegion`] messages.

mod lattice;
mod produce;

pub use lattice::InnerOuterLattice;
pub use produce::{
	produce_lod_refresh_regions, LodRefreshRegions, LodRefreshRegionsError,
	LodRefreshRegionsStatus, LodSceneRefreshRegion, LodSceneRefreshRegionPlugin,
};
