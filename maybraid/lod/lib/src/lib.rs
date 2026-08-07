//! Umbrella crate for generalized LOD in Maybraid ([RFC-154](https://github.com/ramate-io/maybraid/issues/157)).
//!
//! Re-exports [`lod_cascade`] as [`cascade`] and [`lod_cascade_system`] as [`cascade_system`] so dependents can pull in the stack from one dependency when convenient.

pub use lod_cascade as cascade;
pub use lod_cascade_system as cascade_system;

pub mod gen;
pub mod lod_ref;
pub mod presentation;
pub mod scene;

/// Compatibility module paths (prefer [`scene`] / [`presentation`] / [`lod_ref`]).
pub use scene::chunk as scene_chunk;
pub use scene::chunk_fulfill;
pub use scene::cull as lod_cull;
pub use scene::host as lod_scene_host;
pub use scene::level as lod_level;
pub use scene::refresh;
pub use scene::region_index;

pub use lod_ref::{
	collect_node_snapshots, lod_refs_for_bounds, track_lod_nodes, FineLod, LodNode, LodNodePose,
	LodNodeSnapshot, LodRef, LodRequest,
};
pub use presentation::RegionPresenter;
pub use scene::{
	add_lod_refresh_all_for, add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for,
	add_lod_refresh_cull_for, begin_chunk_lod_fulfill, cancel_stale_chunk_fulfillments,
	complete_chunk_lod_fulfill, cull_bands_with_adjacent_depth, cull_lod_level_roots,
	cull_named_from_factor, cull_non_adjacent_bands, cull_offset_bands,
	cull_offset_bands_from_factor, dominant_lod_ref, drain_chunk_lod_fulfill,
	fulfill_lod_level_spawn, lod_host_scene, named_band_index, named_band_progress,
	sync_lod_level_roots, update_lod_host_levels, LodBroadPhasePlugin, LodChunkFulfillBudget,
	LodChunkFulfillment, LodFinePhaseAllPlugin, LodFinePhasePlugin, LodHostBounds, LodLevelRoot,
	LodLevelRootPending, LodLevelRoots, LodLevelSpawnRequest, LodRefresh, LodRefreshCorePlugin,
	LodRefreshCullPlugin, LodRefreshSystems, LodScene, LodSceneCull, LodSceneCulls, LodSceneHost,
	LodSceneHostPlugin, LodSceneLevel, LodSceneRefreshPlugin, LodSceneRefreshRegions,
	LodSceneRegionIndex, LodSceneStatus, LodViewer, LodViewerState, QuantizedDistance, SceneChunk,
	DEFAULT_CHUNK_WEIGHT, NAMED_BANDS_NEAR_TO_FAR, OFFSET_BAND_DEPTH,
};
