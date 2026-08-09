//! Sync: root visibility, level-root fulfill (chunk default / eager optional), cull.

mod chunk;
mod cull;
mod eager;

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::scene::LodScene;

use super::viewer::LodViewer;
use super::LodRefreshSystems;

pub use chunk::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, begin_chunk_lod_fulfill,
	cancel_stale_chunk_fulfillments, complete_chunk_lod_fulfill, drain_chunk_lod_fulfill,
	reset_lod_chunk_budget, LodChunkBudgetClock, LodChunkBudgetPlugin, LodChunkCullSystems,
	LodChunkFulfillBudget, LodChunkFulfillDiag, LodChunkFulfillSystems, LodChunkFulfillment,
	LodLevelRootPending, LodLevelRootStreamed, LodSceneHostStreamed, LodSceneRefreshChunkPlugin,
	LodSceneRefreshSyncPlugin, LodWantsCull,
};
pub use cull::{
	apply_lod_cull_requests, cull_lod_level_roots, drain_lod_cull, enqueue_lod_cull, LodCullEntity,
};
pub use eager::fulfill_lod_level_spawn;

/// Optional eager fulfill + cull (not the default sync path).
pub struct LodSceneRefreshEagerSyncPlugin<T, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, F)>,
}

impl<T, F> Default for LodSceneRefreshEagerSyncPlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<T, F> Plugin for LodSceneRefreshEagerSyncPlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LodChunkBudgetPlugin>() {
			app.add_plugins(LodChunkBudgetPlugin);
		}
		app.add_systems(
			Update,
			(
				fulfill_lod_level_spawn::<T, (), F>
					.in_set(LodRefreshSystems::Fulfill)
					.after(reset_lod_chunk_budget),
				cull_lod_level_roots::<T, (), F>.in_set(LodChunkCullSystems::Enqueue),
			),
		);
	}
}

/// Cull only (probe / external level writers), driven by `F` nodes.
pub struct LodRefreshCullPlugin<T, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, F)>,
}

impl<T, F> Default for LodRefreshCullPlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<T, F> Plugin for LodRefreshCullPlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LodChunkBudgetPlugin>() {
			app.add_plugins(LodChunkBudgetPlugin);
		}
		app.add_systems(
			Update,
			(
				fulfill_lod_level_spawn::<T, (), F>
					.in_set(LodRefreshSystems::Fulfill)
					.after(reset_lod_chunk_budget),
				cull_lod_level_roots::<T, (), F>.in_set(LodChunkCullSystems::Enqueue),
			),
		);
	}
}

/// Register fulfill + cull only (prefer [`LodRefreshCullPlugin`]).
pub fn add_lod_refresh_cull_for<T: Component + LodScene>(app: &mut App) {
	if !app.is_plugin_added::<LodRefreshCullPlugin<T>>() {
		app.add_plugins(LodRefreshCullPlugin::<T>::default());
	}
}
