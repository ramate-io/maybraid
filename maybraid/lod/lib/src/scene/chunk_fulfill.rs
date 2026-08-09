//! Re-export: chunk fulfill lives under [`crate::scene::refresh::sync`].

pub use crate::scene::refresh::sync::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, apply_lod_cull_requests,
	begin_chunk_lod_fulfill, cancel_stale_chunk_fulfillments, complete_chunk_lod_fulfill,
	drain_chunk_lod_fulfill, drain_lod_cull, enqueue_lod_cull, reset_lod_chunk_budget,
	LodChunkBudgetClock, LodChunkBudgetPlugin, LodChunkCullSystems, LodChunkFulfillBudget,
	LodChunkFulfillDiag, LodChunkFulfillSystems, LodChunkFulfillment, LodCullEntity,
	LodLevelRootPending, LodLevelRootStreamed, LodSceneHostStreamed, LodSceneRefreshChunkPlugin,
	LodSceneRefreshSyncPlugin, LodWantsCull,
};
