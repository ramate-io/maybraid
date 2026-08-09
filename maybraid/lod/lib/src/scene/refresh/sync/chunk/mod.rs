//! Incremental LOD level-root fulfillment via [`crate::SceneChunk`].
//!
//! Default sync path (vs optional eager [`super::eager::fulfill_lod_level_spawn`]):
//! builds a pending root, drains weighted primitives under
//! [`LodChunkFulfillBudget`], marks content [`LodLevelRootStreamed`], then
//! completes when next-level nested [`LodSceneHost`]s are [`LodSceneHostStreamed`].
//!
//! Scheduling (begin + drain):
//! - **Presence** (~⅓ budget): cold jobs (no ready root yet).
//! - **Level** (~⅔ budget): warm upgrades, High → Medium → Low → UltraLow.
//! Frame parity swaps which class runs first; leftovers roll into the other.
//! Drain round-robins within each list for fairness.
//!
//! Pipeline (within [`crate::LodRefreshSystems::Fulfill`]):
//! reset budget → cancel/sticky → begin jobs (per `T`) → **one** drain → **one** complete.

mod begin;
mod cancel;
mod complete;
mod drain;
mod schedule;
mod types;
mod util;

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

use crate::scene::LodScene;

use super::super::viewer::LodViewer;
use super::super::{ensure_refresh_core, LodRefreshSystems};
use super::cull::{apply_lod_cull_requests, cull_lod_level_roots, drain_lod_cull, LodCullEntity};

pub use begin::begin_chunk_lod_fulfill;
pub use cancel::cancel_stale_chunk_fulfillments;
pub use complete::complete_chunk_lod_fulfill;
pub use drain::drain_chunk_lod_fulfill;
pub use schedule::reset_lod_chunk_budget;
pub use types::{
	LodChunkBeginClock, LodChunkBudgetClock, LodChunkDrainCursor, LodChunkFulfillBudget,
	LodChunkFulfillDiag, LodChunkFulfillment, LodLevelRootPending, LodLevelRootStreamed,
	LodSceneHostStreamed, LodWantsCull,
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
	/// Mark unwanted roots / hosts ([`cull_lod_level_roots`], cancel already ran in Fulfill).
	Enqueue,
	/// Apply [`LodCullEntity`] → [`LodWantsCull`].
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
	/// Cancel stale pending roots / sticky resume.
	Cancel,
	/// Per-`T` [`begin_chunk_lod_fulfill`].
	Begin,
	/// Shared weighted spawn drain.
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
			.add_message::<LodCullEntity>()
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
					LodChunkFulfillSystems::Cancel,
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
					cancel_stale_chunk_fulfillments.in_set(LodChunkFulfillSystems::Cancel),
					drain_chunk_lod_fulfill.in_set(LodChunkFulfillSystems::Drain),
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

/// Default sync for message-driven refresh: chunk fulfill + cull.
pub struct LodSceneRefreshSyncPlugin<T, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, F)>,
}

impl<T, F> Default for LodSceneRefreshSyncPlugin<T, F>
where
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
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
		app.add_systems(
			Update,
			cull_lod_level_roots::<T, (), F>.in_set(LodChunkCullSystems::Enqueue),
		);
	}
}
