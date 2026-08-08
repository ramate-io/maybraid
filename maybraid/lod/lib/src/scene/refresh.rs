//! LOD refresh + sync driven by region / level messages.
//!
//! Submodules:
//! - [`regions`] — strategy `P` + nodes `F` → [`LodSceneRefreshRegion<M>`]
//! - [`levels`] — region + index → [`LodSceneRefreshLevel`]
//! - [`entities`] — fold max level → write [`crate::LodSceneLevel`]
//! - [`sync`] — root sync, chunk fulfill (default), optional eager fulfill, cull
//!
//! Plugins:
//! - [`LodRefreshCorePlugin`] — sets, node track, root sync (once)
//! - [`LodSceneRefreshRegionPlugin<P, F, M>`] — region production
//! - [`LodSceneRefreshLevelsPlugin<I, M, T, F>`] — level production (+ entities)
//! - [`LodSceneRefreshSyncPlugin<T, F>`] — chunk fulfill + cull
//! - [`LodSceneRefreshPlugin<T, M, I, F>`] — levels + entities + sync (region separate)

mod bounds;
pub mod entities;
pub mod levels;
pub mod regions;
pub mod sync;
mod viewer;

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::lod_ref::track_lod_nodes;
use crate::scene::host::sync_lod_level_roots;
use crate::scene::region_index::LodSceneRegionIndex;
use crate::scene::LodScene;

pub use bounds::LodHostBounds;
pub use entities::{
	dominant_lod_ref, refresh_lod_host_levels, update_lod_host_levels, LodSceneRefreshEntitiesPlugin,
};
pub use levels::{
	produce_lod_refresh_levels, LodSceneRefreshLevel, LodSceneRefreshLevelsPlugin,
};
pub use regions::{
	produce_lod_refresh_regions, InnerOuterLattice, LodRefreshRegions, LodRefreshRegionsError,
	LodRefreshRegionsStatus, LodSceneRefreshRegion, LodSceneRefreshRegionPlugin,
};
pub use sync::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, add_lod_refresh_cull_for,
	begin_chunk_lod_fulfill, cancel_stale_chunk_fulfillments, complete_chunk_lod_fulfill,
	cull_lod_level_roots, drain_chunk_lod_fulfill, fulfill_lod_level_spawn, LodChunkFulfillBudget,
	LodChunkFulfillDiag, LodChunkFulfillment, LodLevelRootPending, LodRefreshCullPlugin,
	LodSceneRefreshChunkPlugin, LodSceneRefreshEagerSyncPlugin, LodSceneRefreshSyncPlugin,
};
pub use viewer::{LodViewer, LodViewerState};

use viewer::sync_lod_viewer_state;

/// System set ordering for refresh + sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodRefreshSystems {
	/// Advance [`crate::lod_ref::LodNodePose`] / mirror [`LodViewerState`].
	Track,
	/// Produce [`LodSceneRefreshRegion`] messages.
	ProduceRegions,
	/// Produce [`LodSceneRefreshLevel`] messages from regions + index.
	ProduceLevels,
	/// Write desired [`crate::LodSceneLevel`] on hosts.
	UpdateLevels,
	/// Show/hide roots and enqueue [`crate::LodLevelSpawnRequest`].
	SyncRoots,
	/// Spawn missing level-root content (chunk default / eager optional).
	Fulfill,
	/// Despawn inactive level roots per [`LodScene::scene_lod_culls`].
	Cull,
}

pub(crate) fn configure_refresh_sets(app: &mut App) {
	app.configure_sets(
		Update,
		(
			LodRefreshSystems::Track,
			LodRefreshSystems::ProduceRegions,
			LodRefreshSystems::ProduceLevels,
			LodRefreshSystems::UpdateLevels,
			LodRefreshSystems::SyncRoots,
			LodRefreshSystems::Fulfill,
			LodRefreshSystems::Cull,
		)
			.chain(),
	);
}

pub(crate) fn ensure_refresh_core(app: &mut App) {
	if !app.is_plugin_added::<LodRefreshCorePlugin>() {
		app.add_plugins(LodRefreshCorePlugin);
	}
}

/// Untyped refresh infrastructure: sets, node tracking, root sync.
pub struct LodRefreshCorePlugin;

impl Plugin for LodRefreshCorePlugin {
	fn build(&self, app: &mut App) {
		configure_refresh_sets(app);
		app.init_resource::<LodViewerState>().add_systems(
			Update,
			(
				track_lod_nodes.in_set(LodRefreshSystems::Track),
				sync_lod_viewer_state.in_set(LodRefreshSystems::Track),
				sync_lod_level_roots.in_set(LodRefreshSystems::SyncRoots),
			),
		);
	}
}

/// Levels + entities + chunk sync for host `T` listening on region channel `M`.
///
/// Add [`LodSceneRefreshRegionPlugin`] separately for region production.
pub struct LodSceneRefreshPlugin<T, M, I, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, M, I, F)>,
}

impl<T, M, I, F> Default for LodSceneRefreshPlugin<T, M, I, F>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<T, M, I, F> Plugin for LodSceneRefreshPlugin<T, M, I, F>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneRegionIndex<T>,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		if !app.is_plugin_added::<LodSceneRefreshLevelsPlugin<I, M, T, F>>() {
			app.add_plugins(LodSceneRefreshLevelsPlugin::<I, M, T, F>::default());
		}
		if !app.is_plugin_added::<LodSceneRefreshSyncPlugin<T, F>>() {
			app.add_plugins(LodSceneRefreshSyncPlugin::<T, F>::default());
		}
	}
}

/// Compatibility alias for [`LodSceneRefreshRegionPlugin`].
pub type LodRefreshProductionPlugin<P, F, M> = LodSceneRefreshRegionPlugin<P, F, M>;
