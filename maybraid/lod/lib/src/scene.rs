//! LOD scene runtime: hosts, levels, refresh, chunks.
//!
//! - [`LodScene`] — how a host type selects and builds LOD content
//! - [`refresh`] — broadphase mark + finephase reload from [`crate::lod_ref::LodNode`]s
//! - [`host`] — ECS hosts / level roots / sync
//! - [`chunk`] / [`chunk_fulfill`] — amortized level-root spawn
//!
//! Driver refs live in [`crate::lod_ref`] (not scene-specific).

pub mod chunk;
pub mod chunk_fulfill;
pub mod cull;
pub mod host;
pub mod level;
pub mod lod_scene;
pub mod refresh;
pub mod region_index;

pub use chunk::{SceneChunk, DEFAULT_CHUNK_WEIGHT};
pub use chunk_fulfill::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, begin_chunk_lod_fulfill,
	cancel_stale_chunk_fulfillments, complete_chunk_lod_fulfill, drain_chunk_lod_fulfill,
	LodChunkFulfillBudget, LodChunkFulfillment, LodLevelRootPending,
};
pub use cull::{
	cull_bands_with_adjacent_depth, cull_named_from_factor, cull_non_adjacent_bands,
	cull_offset_bands, cull_offset_bands_from_factor, named_band_index, named_band_progress,
	LodSceneCull, LodSceneCulls, NAMED_BANDS_NEAR_TO_FAR, OFFSET_BAND_DEPTH,
};
pub use host::{
	lod_host_scene, sync_lod_level_roots, LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest,
	LodSceneHost, LodSceneHostPlugin,
};
pub use level::{LodSceneLevel, QuantizedDistance};
pub use lod_scene::{LodScene, LodSceneStatus};
pub use refresh::{
	add_lod_refresh_all_for, add_lod_refresh_cull_for, cull_lod_level_roots, dominant_lod_ref,
	fulfill_lod_level_spawn, produce_lod_refresh_regions, update_lod_host_levels,
	InnerOuterLattice, LodBroadPhasePlugin, LodFinePhaseAllPlugin, LodFinePhasePlugin,
	LodHostBounds, LodRefresh, LodRefreshCorePlugin, LodRefreshCullPlugin, LodRefreshProductionPlugin,
	LodRefreshRegions, LodRefreshRegionsError, LodRefreshRegionsOutlet, LodRefreshRegionsStatus,
	LodRefreshSystems, LodSceneRefreshPlugin, LodSceneRefreshRegions, LodViewer, LodViewerState,
};
pub use region_index::LodSceneRegionIndex;
