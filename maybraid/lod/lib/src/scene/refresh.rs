//! LOD refresh: broadphase region mark + finephase host reload from [`LodNode`]s.
//!
//! Plugin layers:
//! - [`LodRefreshCorePlugin`] — sets, node track, root sync (untyped, once)
//! - [`LodRefreshRegionsPlugin<P, F, M>`] — produce regions from nodes `F` → outlet `M`
//! - [`LodBroadPhasePlugin<T, M, I>`] — stamp [`LodRefresh`] from regions on `M`
//! - [`LodFinePhasePlugin<T, F>`] — update/fulfill/cull `(T, LodRefresh)` vs nodes `F`
//! - [`LodSceneRefreshPlugin<T, M, I, F>`] — compose Core + Broad + Fine
//!
//! Ephemeral [`crate::lod_ref::LodRef`]s are built from [`crate::lod_ref::LodNode`] /
//! [`crate::lod_ref::LodNodePose`] + [`LodHostBounds`]. [`LodViewerState`] remains a
//! probe compatibility mirror.

mod bounds;
mod cull;
mod fulfill;
mod mark;
mod regions;
mod update;
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
pub(crate) use bounds::ephemeral_bounds;
pub use cull::cull_lod_level_roots;
pub use fulfill::fulfill_lod_level_spawn;
pub use mark::{
	clear_coarse_lod_refresh, mark_lod_refresh_from_regions, LodRefresh, LodSceneRefreshRegions,
};
pub use regions::{
	produce_lod_refresh_regions, LodRefreshRegions, LodRefreshRegionsOutlet,
	LodRefreshRegionsStatus,
};
pub use update::{dominant_lod_ref, update_lod_host_levels};
pub use viewer::{LodViewer, LodViewerState};

use viewer::sync_lod_viewer_state;

/// System set ordering for the refresh pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodRefreshSystems {
	/// Advance [`LodNodePose`] / mirror [`LodViewerState`].
	Track,
	/// Produce [`LodSceneRefreshRegions`] on marker-scoped outlets.
	ProduceRegions,
	/// Stamp [`LodRefresh`] from marker-scoped [`LodSceneRefreshRegions`].
	Mark,
	/// Write desired [`crate::LodSceneLevel`] on hosts.
	UpdateLevels,
	/// Show/hide roots and enqueue [`crate::LodLevelSpawnRequest`].
	SyncRoots,
	/// Spawn missing level-root content for hosts with a spawn request.
	Fulfill,
	/// Despawn inactive level roots per [`LodScene::scene_lod_culls`].
	Cull,
	/// Drop [`LodRefresh::Coarse`] after Cull.
	ClearCoarse,
}

pub(crate) fn configure_refresh_sets(app: &mut App) {
	app.configure_sets(
		Update,
		(
			LodRefreshSystems::Track,
			LodRefreshSystems::ProduceRegions,
			LodRefreshSystems::Mark,
			LodRefreshSystems::UpdateLevels,
			LodRefreshSystems::SyncRoots,
			LodRefreshSystems::Fulfill,
			LodRefreshSystems::Cull,
			LodRefreshSystems::ClearCoarse,
		)
			.chain(),
	);
}

fn ensure_refresh_core(app: &mut App) {
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

/// Produce [`LodSceneRefreshRegions`] on a stable `M` outlet from `F`-filtered [`LodNode`]s.
///
/// `P` is a [`Resource`] implementing [`LodRefreshRegions`] (`init_resource` on add).
pub struct LodRefreshRegionsPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Component + Default + 'static,
{
	_marker: PhantomData<fn() -> (P, F, M)>,
}

impl<P, F, M> Default for LodRefreshRegionsPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Component + Default + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<P, F, M> Plugin for LodRefreshRegionsPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Component + Default + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.init_resource::<P>().add_systems(
			Update,
			produce_lod_refresh_regions::<P, F, M>.in_set(LodRefreshSystems::ProduceRegions),
		);
	}
}

/// Broadphase: stamp [`LodRefresh`] on hosts `T` from [`LodSceneRefreshRegions`] on `M`.
pub struct LodBroadPhasePlugin<T, M, I>
where
	T: Component + LodScene + 'static,
	M: Component + 'static,
	I: SystemParam + 'static,
{
	_marker: PhantomData<fn() -> (T, M, I)>,
}

impl<T, M, I> Default for LodBroadPhasePlugin<T, M, I>
where
	T: Component + LodScene + 'static,
	M: Component + 'static,
	I: SystemParam + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<T, M, I> Plugin for LodBroadPhasePlugin<T, M, I>
where
	T: Component + LodScene + 'static,
	M: Component + 'static,
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneRegionIndex<T>,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_systems(
			Update,
			(
				mark_lod_refresh_from_regions::<T, M, I>.in_set(LodRefreshSystems::Mark),
				clear_coarse_lod_refresh::<T>.in_set(LodRefreshSystems::ClearCoarse),
			),
		);
	}
}

/// Finephase: reload hosts `(T, LodRefresh)` from [`LodNode`]s filtered by `F`.
pub struct LodFinePhasePlugin<T, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, F)>,
}

impl<T, F> Default for LodFinePhasePlugin<T, F>
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

impl<T, F> Plugin for LodFinePhasePlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_systems(
			Update,
			(
				update_lod_host_levels::<T, With<LodRefresh>, F>
					.in_set(LodRefreshSystems::UpdateLevels),
				fulfill_lod_level_spawn::<T, With<LodRefresh>, F>
					.in_set(LodRefreshSystems::Fulfill),
				cull_lod_level_roots::<T, With<LodRefresh>, F>.in_set(LodRefreshSystems::Cull),
			),
		);
	}
}

/// Compose Core + BroadPhase + FinePhase for one host / region / node binding.
pub struct LodSceneRefreshPlugin<T, M, I, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	M: Component + 'static,
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, M, I, F)>,
}

impl<T, M, I, F> Default for LodSceneRefreshPlugin<T, M, I, F>
where
	T: Component + LodScene + 'static,
	M: Component + 'static,
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
	M: Component + 'static,
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneRegionIndex<T>,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		if !app.is_plugin_added::<LodBroadPhasePlugin<T, M, I>>() {
			app.add_plugins(LodBroadPhasePlugin::<T, M, I>::default());
		}
		if !app.is_plugin_added::<LodFinePhasePlugin<T, F>>() {
			app.add_plugins(LodFinePhasePlugin::<T, F>::default());
		}
	}
}

/// Unscoped finephase: all hosts `T`, driven by `F`-filtered [`LodNode`]s (no [`LodRefresh`]).
pub struct LodFinePhaseAllPlugin<T, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, F)>,
}

impl<T, F> Default for LodFinePhaseAllPlugin<T, F>
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

impl<T, F> Plugin for LodFinePhaseAllPlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_systems(
			Update,
			(
				update_lod_host_levels::<T, (), F>.in_set(LodRefreshSystems::UpdateLevels),
				fulfill_lod_level_spawn::<T, (), F>.in_set(LodRefreshSystems::Fulfill),
				cull_lod_level_roots::<T, (), F>.in_set(LodRefreshSystems::Cull),
			),
		);
	}
}

/// Fulfill + cull only (probe / external level writers), driven by `F` nodes.
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
		ensure_refresh_core(app);
		app.add_systems(
			Update,
			(
				fulfill_lod_level_spawn::<T, (), F>.in_set(LodRefreshSystems::Fulfill),
				cull_lod_level_roots::<T, (), F>.in_set(LodRefreshSystems::Cull),
			),
		);
	}
}

/// Register unscoped update + fulfill + cull (prefer [`LodFinePhaseAllPlugin`]).
pub fn add_lod_refresh_all_for<T: Component + LodScene>(app: &mut App) {
	if !app.is_plugin_added::<LodFinePhaseAllPlugin<T>>() {
		app.add_plugins(LodFinePhaseAllPlugin::<T>::default());
	}
}

/// Register fulfill + cull only (prefer [`LodRefreshCullPlugin`]).
pub fn add_lod_refresh_cull_for<T: Component + LodScene>(app: &mut App) {
	if !app.is_plugin_added::<LodRefreshCullPlugin<T>>() {
		app.add_plugins(LodRefreshCullPlugin::<T>::default());
	}
}
