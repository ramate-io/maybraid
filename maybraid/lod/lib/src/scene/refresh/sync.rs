//! Sync: root visibility, level-root fulfill (chunk), cull.

mod chunk;
mod cull;

use bevy::prelude::*;

use crate::scene::LodScene;

use super::viewer::LodViewer;

pub use chunk::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, begin_chunk_lod_fulfill,
	cancel_unstarted_cull_for_desired_pending_roots, complete_chunk_lod_fulfill,
	drain_chunk_lod_fulfill, reset_lod_chunk_budget, FulfillClass,
	LodChunkBeginClock, LodChunkBudgetClock, LodChunkBudgetPlugin, LodChunkCullSystems,
	LodChunkDrainCursor, LodChunkFulfillBudget, LodChunkFulfillSystems,
	LodChunkFulfillment, LodCullInFlight, LodLevelRootPending, LodLevelRootStreamed,
	LodSceneHostStreamed, LodSceneRefreshChunkPlugin, LodSceneRefreshSyncPlugin,
};
pub use cull::{
	apply_lod_cull_requests, cull_lod_level_roots, drain_lod_cull, enqueue_lod_cull, LodCullRequest,
};

/// Chunk fulfill + full-scan cull for hosts whose levels are written elsewhere (e.g. probes).
///
/// Prefer message-driven [`LodSceneRefreshSyncPlugin`] / Avian region plugins when possible.
/// For viewer-distance level writes + chunk + cull, use [`add_lod_refresh_chunk_full_for`].
pub fn add_lod_refresh_cull_for<T: Component + LodScene>(app: &mut App) {
	if !app.is_plugin_added::<LodChunkBudgetPlugin>() {
		app.add_plugins(LodChunkBudgetPlugin);
	}
	app.add_systems(
		Update,
		cull_lod_level_roots::<T, (), With<LodViewer>>.in_set(LodChunkCullSystems::Enqueue),
	);
	add_lod_refresh_chunk_for::<T>(app);
}
