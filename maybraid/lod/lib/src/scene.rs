//! LOD scene runtime: hosts, levels, refresh, chunks.
//!
//! - [`LodScene`] — how a host type selects and builds LOD content
//! - [`refresh`] — region / level messages → host levels → sync
//! - [`host`] — ECS hosts / level roots / sync
//! - [`chunk`] / [`chunk_fulfill`] — amortized level-root spawn
//!
//! Driver refs live in [`crate::lod_ref`] (not scene-specific).

pub mod bounds_patch;
pub mod chunk;
pub mod chunk_fulfill;
pub mod cull;
pub mod host;
pub mod level;
pub mod lod_scene;
pub mod refresh;
pub mod region_index;

pub use bounds_patch::{LodSceneBoundsMarshaller, PatchSceneBounds};
pub use chunk::{SceneChunk, DEFAULT_CHUNK_WEIGHT};
pub use chunk_fulfill::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, apply_lod_cull_requests,
	begin_chunk_lod_fulfill, cancel_unstarted_cull_for_desired_pending_roots,
	complete_chunk_lod_fulfill, drain_chunk_lod_fulfill, drain_lod_cull, enqueue_lod_cull,
	reset_lod_chunk_budget, LodChunkBudgetClock, LodChunkBudgetPlugin, LodChunkCullSystems,
	LodChunkFulfillBudget, LodChunkFulfillSystems, LodChunkFulfillment, LodCullInFlight,
	LodCullRequest, LodLevelRootPending, LodLevelRootStreamed, LodSceneHostStreamed,
	LodSceneRefreshChunkPlugin, LodSceneRefreshSyncPlugin,
};
pub use cull::{
	closest_available_lod_level, cull_bands_with_adjacent_depth, cull_named_from_factor,
	cull_non_adjacent_bands, cull_offset_bands, cull_offset_bands_from_factor, named_band_index,
	named_band_progress, LodSceneCull, LodSceneCulls, NAMED_BANDS_NEAR_TO_FAR, OFFSET_BAND_DEPTH,
};
pub use host::{
	host_shows_level_root, lod_host_scene, lod_host_scene_pending, lod_level_roots_entity,
	lod_root_is_shown, nested_host_parent_allows_refresh, parent_host_desired_or_high,
	sync_lod_level_roots, LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest, LodSceneHost,
	LodSceneHostPlugin,
};
pub use level::{LodSceneLevel, QuantizedDistance};
pub use lod_scene::{LodScene, LodSceneStatus};
pub use refresh::{
	add_lod_refresh_cull_for, cull_lod_level_roots, dominant_lod_ref, produce_lod_cull_for_region,
	produce_lod_cull_regions, produce_lod_refresh_levels, produce_lod_refresh_regions,
	refresh_lod_host_levels, sync_cullable_roots_marker, sync_nested_refresh_allowed,
	update_lod_host_levels, Bullseye, LodCullMarkerPlugin, LodCullRegionCursor, LodCullRegions,
	LodCullRegionsStatus, LodHostBounds, LodHostHasCullableRoots, LodNestedRefreshAllowed,
	LodNestedRefreshBlocked, LodRefreshCorePlugin, LodRefreshProductionPlugin, LodRefreshRegions,
	LodRefreshRegionsError, LodRefreshRegionsStatus, LodRefreshSystems, LodSceneCullRegion,
	LodSceneCullRegionPlugin, LodSceneRefreshEntitiesPlugin, LodSceneRefreshLevel,
	LodSceneRefreshLevelsPlugin, LodSceneRefreshPlugin, LodSceneRefreshRegion,
	LodSceneRefreshRegionPlugin, LodSceneRegionCullPlugin, LodViewer, OpenLattice, Spotlight,
};
pub use region_index::LodSceneRegionIndex;
