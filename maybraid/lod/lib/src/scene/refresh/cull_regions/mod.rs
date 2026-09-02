//! Region-scoped cull production (rotating lattice) + enqueue.
//!
//! Parallel to refresh regions/levels, but:
//! - producers may emit every frame (camera still)
//! - a [`LodCullRegionCursor`] round-robins annulus tiles (e.g. [`OpenLattice`])
//! - fill is one untyped host-hit query; enqueue dispatches through erased producers

mod cache;
mod cursor;
mod enqueue;
mod markers;
mod open_lattice;
mod produce;

pub use cache::{
	fill_lod_cull_produce_cache, LodCullProduceCache, LodSceneCullAabb,
	LodSceneCullProduceFillPlugin,
};
pub use cursor::LodCullRegionCursor;
pub use enqueue::{
	produce_lod_cull_for_region, produce_lod_cull_for_region_erased, LodSceneRegionCullPlugin,
};
pub use markers::{
	sync_cullable_roots_marker, sync_nested_refresh_allowed, LodCullMarkerPlugin,
	LodHostHasCullableRoots, LodNestedRefreshAllowed, LodNestedRefreshBlocked,
	LodNestedRefreshSyncBudget,
};
pub use open_lattice::OpenLattice;
pub use produce::{
	produce_lod_cull_regions, LodCullRegions, LodCullRegionsStatus, LodSceneCullRegion,
	LodSceneCullRegionPlugin,
};
