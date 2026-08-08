//! Produce [`LodSceneRefreshLevel`] from region impulses and a spatial index.

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::prelude::*;

use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose,
};
use crate::scene::level::LodSceneLevel;
use crate::scene::region_index::LodSceneRegionIndex;
use crate::scene::LodScene;

use super::super::regions::LodSceneRefreshRegion;
use super::super::viewer::LodViewer;
use super::super::{ensure_refresh_core, LodRefreshSystems};

/// Impulse: set host `entity` toward `level` (folded by max in entity refresh).
#[derive(Message, Debug, Clone, Copy)]
pub struct LodSceneRefreshLevel {
	pub entity: Entity,
	pub level: LodSceneLevel,
}

/// For each [`LodSceneRefreshRegion<M>`], query hosts `T` via `I` and emit levels
/// from `F`-filtered [`LodNode`]s.
///
/// [`LodRef`] bounds come from each node's [`LodNodeBounds`] (or a point), not from
/// the host.
pub fn produce_lod_refresh_levels<I, M, T, F>(
	mut regions: MessageReader<LodSceneRefreshRegion<M>>,
	index: StaticSystemParam<I>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	mut levels: MessageWriter<LodSceneRefreshLevel>,
) where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneRegionIndex<T>,
	M: Send + Sync + 'static,
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	let snapshots = collect_node_snapshots(&nodes);
	if snapshots.is_empty() {
		return;
	}
	let refs = lod_refs_from_snapshots(&snapshots);
	let ref_refs: Vec<_> = refs.iter().collect();

	let index = index.into_inner();
	for region_msg in regions.read() {
		for (entity, scene) in index.hosts_in_region(region_msg.region) {
			let level = scene.scene_lod_level_from_levels(&ref_refs);
			levels.write(LodSceneRefreshLevel { entity, level });
		}
	}
}

/// Region messages `M` + index `I` → [`LodSceneRefreshLevel`] for hosts `T`.
pub struct LodSceneRefreshLevelsPlugin<I, M, T, F = With<LodViewer>>
where
	I: SystemParam + 'static,
	M: Send + Sync + 'static,
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (I, M, T, F)>,
}

impl<I, M, T, F> Default for LodSceneRefreshLevelsPlugin<I, M, T, F>
where
	I: SystemParam + 'static,
	M: Send + Sync + 'static,
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<I, M, T, F> Plugin for LodSceneRefreshLevelsPlugin<I, M, T, F>
where
	I: SystemParam + 'static,
	M: Send + Sync + 'static,
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneRegionIndex<T>,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		if !app.is_plugin_added::<super::super::entities::LodSceneRefreshEntitiesPlugin<T>>() {
			app.add_plugins(super::super::entities::LodSceneRefreshEntitiesPlugin::<T>::default());
		}
		app.add_message::<LodSceneRefreshRegion<M>>()
			.add_message::<LodSceneRefreshLevel>()
			.add_systems(
				Update,
				produce_lod_refresh_levels::<I, M, T, F>.in_set(LodRefreshSystems::ProduceLevels),
			);
	}
}
