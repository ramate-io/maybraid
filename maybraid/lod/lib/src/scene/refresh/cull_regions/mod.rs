//! Region-scoped cull production (rotating lattice) + enqueue.
//!
//! Parallel to refresh regions/levels, but:
//! - producers may emit every frame (camera still)
//! - a [`LodCullRegionCursor`] round-robins annulus tiles (e.g. [`OpenLattice`])
//! - enqueue uses [`crate::LodSceneRegionIndex`] instead of scanning all hosts

mod cursor;
mod enqueue;
mod markers;
mod open_lattice;
mod produce;

pub use cursor::LodCullRegionCursor;
pub use enqueue::{produce_lod_cull_for_region, LodSceneRegionCullPlugin};
pub use markers::{
	sync_cullable_roots_marker, sync_nested_refresh_allowed, LodCullMarkerPlugin,
	LodHostHasCullableRoots, LodNestedRefreshAllowed, LodNestedRefreshBlocked,
};
pub use open_lattice::OpenLattice;
pub use produce::{
	produce_lod_cull_regions, LodCullRegions, LodCullRegionsStatus, LodSceneCullRegion,
	LodSceneCullRegionPlugin,
};
