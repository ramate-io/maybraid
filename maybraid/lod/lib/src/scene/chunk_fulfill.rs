//! Re-export: chunk fulfill lives under [`crate::scene::refresh::sync`].

pub use crate::scene::refresh::sync::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, begin_chunk_lod_fulfill,
	cancel_stale_chunk_fulfillments, complete_chunk_lod_fulfill, drain_chunk_lod_fulfill,
	LodChunkFulfillBudget, LodChunkFulfillDiag, LodChunkFulfillment, LodLevelRootPending,
	LodSceneRefreshChunkPlugin, LodSceneRefreshSyncPlugin,
};
