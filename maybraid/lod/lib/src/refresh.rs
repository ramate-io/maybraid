//! LOD refresh pass: viewer track → region mark → update → sync → fulfill → cull.
//!
//! Higher-order cascade / producers write [`LodSceneRefreshRegions`]; this pass
//! stamps [`LodRefresh`] and runs level sync / fulfill / cull on the marked set.
//!
//! Pipeline (no camera types here):
//! [`LodViewer`] → [`LodViewerState`] → [`mark::mark_lod_refresh_from_regions`] →
//! [`update_lod_host_levels`] → [`crate::sync_lod_level_roots`] →
//! [`fulfill_lod_level_spawn`] → [`cull_lod_level_roots`] →
//! [`mark::clear_coarse_lod_refresh`].
//!
//! Construct [`crate::LodRef`] ephemerally from [`LodViewerState`] + [`LodHostBounds`].

mod bounds;
mod cull;
mod fulfill;
mod mark;
mod update;
mod viewer;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::gen::LodScene;
use crate::lod_scene_host::sync_lod_level_roots;
use crate::region_index::LodSceneRegionIndex;

pub use bounds::LodHostBounds;
pub(crate) use bounds::ephemeral_bounds;
pub use cull::cull_lod_level_roots;
pub use fulfill::fulfill_lod_level_spawn;
pub use mark::{
	clear_coarse_lod_refresh, mark_lod_refresh_from_regions, LodRefresh, LodSceneRefreshRegions,
};
pub use update::update_lod_host_levels;
pub use viewer::{track_lod_viewer, LodViewer, LodViewerState};

/// System set ordering for the refresh pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodRefreshSystems {
	/// Copy [`LodViewer`] transforms into [`LodViewerState`].
	Track,
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

/// Initializes [`LodViewerState`], tracks [`LodViewer`], and schedules root sync.
///
/// Register per-type work with [`add_lod_refresh_for`], [`add_lod_refresh_all_for`],
/// or [`add_lod_refresh_cull_for`].
pub struct LodRefreshPlugin;

impl Plugin for LodRefreshPlugin {
	fn build(&self, app: &mut App) {
		configure_refresh_sets(app);
		app.init_resource::<LodViewerState>().add_systems(
			Update,
			(
				track_lod_viewer.in_set(LodRefreshSystems::Track),
				sync_lod_level_roots.in_set(LodRefreshSystems::SyncRoots),
			),
		);
	}
}

/// Region-scoped refresh for host `T` listening to [`LodSceneRefreshRegions`] on `M`.
///
/// `I` is a [`SystemParam`] whose item implements [`LodSceneRegionIndex<T>`].
/// Update / fulfill / cull run only on hosts with [`LodRefresh`]; coarse markers
/// clear after Cull, fine markers stay.
pub fn add_lod_refresh_for<T, M, I>(app: &mut App)
where
	T: Component + LodScene + 'static,
	M: Component + 'static,
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneRegionIndex<T>,
{
	configure_refresh_sets(app);
	app.add_systems(
		Update,
		(
			mark_lod_refresh_from_regions::<T, M, I>.in_set(LodRefreshSystems::Mark),
			update_lod_host_levels::<T, With<LodRefresh>>.in_set(LodRefreshSystems::UpdateLevels),
			fulfill_lod_level_spawn::<T, With<LodRefresh>>.in_set(LodRefreshSystems::Fulfill),
			cull_lod_level_roots::<T, With<LodRefresh>>.in_set(LodRefreshSystems::Cull),
			clear_coarse_lod_refresh::<T>.in_set(LodRefreshSystems::ClearCoarse),
		),
	);
}

/// Unscoped refresh: update + fulfill + cull for all hosts of type `T`.
///
/// Prefer [`add_lod_refresh_for`] once a region producer is wired.
pub fn add_lod_refresh_all_for<T: Component + LodScene>(app: &mut App) {
	configure_refresh_sets(app);
	app.add_systems(
		Update,
		(
			update_lod_host_levels::<T, ()>.in_set(LodRefreshSystems::UpdateLevels),
			fulfill_lod_level_spawn::<T, ()>.in_set(LodRefreshSystems::Fulfill),
			cull_lod_level_roots::<T, ()>.in_set(LodRefreshSystems::Cull),
		),
	);
}

/// Fulfill + cull for hosts that already update [`crate::LodSceneLevel`] elsewhere.
pub fn add_lod_refresh_cull_for<T: Component + LodScene>(app: &mut App) {
	configure_refresh_sets(app);
	app.add_systems(
		Update,
		(
			fulfill_lod_level_spawn::<T, ()>.in_set(LodRefreshSystems::Fulfill),
			cull_lod_level_roots::<T, ()>.in_set(LodRefreshSystems::Cull),
		),
	);
}
