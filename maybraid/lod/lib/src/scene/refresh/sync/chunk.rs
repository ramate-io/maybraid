//! Incremental LOD level-root fulfillment via [`crate::SceneChunk`].
//!
//! Default sync path for level-root fulfillment:
//! builds a pending root, drains weighted primitives under
//! [`LodChunkFulfillBudget`], marks content [`LodLevelRootStreamed`], then
//! completes when next-level nested [`LodSceneHost`]s are [`LodSceneHostStreamed`].
//!
//! Scheduling (begin + drain):
//! - **Presence** (~¼): cold jobs (empty → something).
//! - **Desired** (~⅜): warm jobs for the host's desired level root.
//! - **Active** (~⅜): warm jobs on a shown (non-Hidden) non-desired root — warm-hold.
//! Drain ranks `(parent_desired, self_level)` High→… within each class.
//! Begin uses Presence + Desired (Active begin quota folds into Desired).
//! Frame parity rotates class order; leftovers cascade.
//!
//! Pipeline (within [`crate::LodRefreshSystems::Fulfill`]):
//! reset budget → resume desired cull-inflight → begin (per `T`) → drain → complete.

mod begin;
mod complete;
mod drain;
mod resume;
mod schedule;
mod types;
mod util;

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::scene::LodScene;

use super::super::viewer::LodViewer;
use super::super::{ensure_refresh_core, LodRefreshSystems};
use super::cull::{
	apply_lod_cull_requests, cull_lod_level_roots, drain_lod_cull, LodCullRequest,
};

pub use begin::begin_chunk_lod_fulfill;
pub use complete::{bump_nested_streamed_progress, complete_chunk_lod_fulfill};
pub use drain::drain_chunk_lod_fulfill;
pub use resume::resume_desired_pending_roots;
pub use schedule::reset_lod_chunk_budget;
pub use types::{
	FulfillClass, LodChunkBeginClock, LodChunkBudgetClock, LodChunkDrainCursor,
	LodChunkFulfillBudget, LodChunkFulfillDiag, LodChunkFulfillment, LodCullInFlight,
	LodLevelRootPending, LodLevelRootStreamed, LodSceneHostStreamed,
};

/// Register incremental chunk fulfill systems for one [`LodScene`] host type.
pub fn add_lod_refresh_chunk_for<T: Component + LodScene>(app: &mut App) {
	if !app.is_plugin_added::<LodSceneRefreshChunkPlugin<T>>() {
		app.add_plugins(LodSceneRefreshChunkPlugin::<T>::default());
	}
}

/// Chunk fulfill plugin (default sync path for level-root spawn).
pub struct LodSceneRefreshChunkPlugin<T>
where
	T: Component + LodScene + 'static,
{
	_marker: PhantomData<fn() -> T>,
}

impl<T> Default for LodSceneRefreshChunkPlugin<T>
where
	T: Component + LodScene + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

/// Substeps within [`LodRefreshSystems::Cull`] (order against these, not system types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodChunkCullSystems {
	/// Mark unwanted roots / hosts ([`cull_lod_level_roots`] / region cull).
	Enqueue,
	/// Apply [`LodCullRequest`] → [`LodCullInFlight`].
	Apply,
	/// Budgeted leaf-first despawn.
	Drain,
}

/// Substeps within [`LodRefreshSystems::Fulfill`] after budget reset.
///
/// [`Self::Drain`] / [`Self::Complete`] are registered **once** (shared). Per-host-type
/// plugins only add [`begin_chunk_lod_fulfill`] into [`Self::Begin`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodChunkFulfillSystems {
	/// Resume desired pending roots that had a cull request before teardown started.
	Resume,
	/// Per-`T` [`begin_chunk_lod_fulfill`].
	Begin,
	/// Shared weighted spawn drain (desired jobs only).
	Drain,
	/// Shared warm-swap complete.
	Complete,
}

/// Shared chunk budget clock, cull messages, and one-shot drain/complete registration.
pub struct LodChunkBudgetPlugin;

impl Plugin for LodChunkBudgetPlugin {
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.init_resource::<LodChunkFulfillBudget>()
			.init_resource::<LodChunkBudgetClock>()
			.init_resource::<LodChunkBeginClock>()
			.init_resource::<LodChunkDrainCursor>()
			.init_resource::<LodChunkFulfillDiag>()
			.add_message::<LodCullRequest>()
			.configure_sets(
				Update,
				(
					LodChunkCullSystems::Enqueue,
					LodChunkCullSystems::Apply,
					LodChunkCullSystems::Drain,
				)
					.chain()
					.in_set(LodRefreshSystems::Cull),
			)
			.configure_sets(
				Update,
				(
					LodChunkFulfillSystems::Resume,
					LodChunkFulfillSystems::Begin,
					LodChunkFulfillSystems::Drain,
					LodChunkFulfillSystems::Complete,
				)
					.chain()
					.in_set(LodRefreshSystems::Fulfill)
					.after(reset_lod_chunk_budget),
			)
			.add_systems(
				Update,
				(
					reset_lod_chunk_budget.in_set(LodRefreshSystems::Fulfill),
					resume_desired_pending_roots.in_set(LodChunkFulfillSystems::Resume),
					drain_chunk_lod_fulfill.in_set(LodChunkFulfillSystems::Drain),
					bump_nested_streamed_progress
						.in_set(LodChunkFulfillSystems::Complete)
						.before(complete_chunk_lod_fulfill),
					complete_chunk_lod_fulfill.in_set(LodChunkFulfillSystems::Complete),
					apply_lod_cull_requests.in_set(LodChunkCullSystems::Apply),
					drain_lod_cull.in_set(LodChunkCullSystems::Drain),
				),
			);
	}
}

fn ensure_chunk_budget(app: &mut App) {
	if !app.is_plugin_added::<LodChunkBudgetPlugin>() {
		app.add_plugins(LodChunkBudgetPlugin);
	}
}

impl<T> Plugin for LodSceneRefreshChunkPlugin<T>
where
	T: Component + LodScene + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_chunk_budget(app);
		app.add_systems(Update, begin_chunk_lod_fulfill::<T>.in_set(LodChunkFulfillSystems::Begin));
	}
}

/// Probe-style levels + chunk fulfill + cull (no region message pipeline).
pub fn add_lod_refresh_chunk_full_for<T: Component + LodScene>(app: &mut App) {
	ensure_chunk_budget(app);
	app.add_systems(
		Update,
		(
			crate::scene::refresh::update_lod_host_levels::<T, (), With<LodViewer>>
				.in_set(LodRefreshSystems::UpdateLevels),
			cull_lod_level_roots::<T, (), With<LodViewer>>.in_set(LodChunkCullSystems::Enqueue),
		),
	);
	add_lod_refresh_chunk_for::<T>(app);
}

/// Default sync for message-driven refresh: chunk fulfill + optional full-scan cull.
///
/// Prefer [`Self::without_full_scan_cull`] when using
/// [`crate::scene::refresh::cull_regions::LodSceneRegionCullPlugin`].
pub struct LodSceneRefreshSyncPlugin<T, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	full_scan_cull: bool,
	_marker: PhantomData<fn() -> (T, F)>,
}

impl<T, F> Default for LodSceneRefreshSyncPlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self {
			full_scan_cull: true,
			_marker: PhantomData,
		}
	}
}

impl<T, F> LodSceneRefreshSyncPlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	/// Chunk fulfill only — region cull plugins own enqueue.
	pub fn without_full_scan_cull() -> Self {
		Self {
			full_scan_cull: false,
			_marker: PhantomData,
		}
	}
}

impl<T, F> Plugin for LodSceneRefreshSyncPlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_chunk_budget(app);
		if !app.is_plugin_added::<LodSceneRefreshChunkPlugin<T>>() {
			app.add_plugins(LodSceneRefreshChunkPlugin::<T>::default());
		}
		if self.full_scan_cull {
			app.add_systems(
				Update,
				cull_lod_level_roots::<T, (), F>.in_set(LodChunkCullSystems::Enqueue),
			);
		}
	}
}
