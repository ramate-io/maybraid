//! Umbrella crate for generalized LOD in Maybraid ([RFC-154](https://github.com/ramate-io/maybraid/issues/157)).
//!
//! Re-exports [`lod_cascade`] as [`cascade`] and [`lod_cascade_system`] as [`cascade_system`] so dependents can pull in the stack from one dependency when convenient.

pub use lod_cascade as cascade;
pub use lod_cascade_system as cascade_system;
pub mod chunk_fulfill;
pub mod fine_pass;
pub mod gen;
pub mod lod_cull;
pub mod lod_level;
pub mod lod_ref;
pub mod lod_scene_host;
pub mod region_index;
pub mod scene_chunk;

pub use chunk_fulfill::{
	add_fine_pass_chunk_for, add_fine_pass_chunk_full_for, begin_chunk_lod_fulfill,
	cancel_stale_chunk_fulfillments, complete_chunk_lod_fulfill, drain_chunk_lod_fulfill,
	LodChunkFulfillBudget, LodChunkFulfillment, LodLevelRootPending,
};
pub use fine_pass::{
	add_fine_pass_cull_for, add_fine_pass_for, cull_lod_level_roots, fulfill_lod_level_spawn,
	track_lod_viewer, update_lod_host_levels, LodFinePassPlugin, LodFinePassSystems,
	LodHostBounds, LodViewer, LodViewerState,
};
pub use lod_cull::{
	cull_bands_with_adjacent_depth, cull_named_from_factor, cull_non_adjacent_bands,
	cull_offset_bands, cull_offset_bands_from_factor, named_band_index, named_band_progress,
	LodSceneCull, LodSceneCulls, NAMED_BANDS_NEAR_TO_FAR, OFFSET_BAND_DEPTH,
};
pub use lod_level::{LodSceneLevel, QuantizedDistance};
pub use lod_scene_host::{
	lod_host_scene, sync_lod_level_roots, LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest,
	LodSceneHost, LodSceneHostPlugin,
};
pub use region_index::LodSceneRegionIndex;
pub use scene_chunk::{SceneChunk, DEFAULT_CHUNK_WEIGHT};
