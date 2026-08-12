//! Region-scoped cull enqueue via [`crate::LodSceneRegionIndex`].

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::prelude::*;

use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose,
};
use crate::scene::cull::LodSceneCulls;
use crate::scene::host::{LodLevelRoot, LodLevelRoots, LodSceneHost};
use crate::scene::level::LodSceneLevel;
use crate::scene::region_index::LodSceneRegionIndex;
use crate::scene::LodScene;

use super::super::sync::{
	enqueue_lod_cull, LodChunkBudgetPlugin, LodChunkCullSystems, LodCullRequest,
	LodLevelRootPending, LodCullInFlight,
};
use super::super::viewer::LodViewer;
use super::super::ensure_refresh_core;
use super::markers::{LodCullMarkerPlugin, LodHostHasCullableRoots, LodNestedRefreshAllowed};
use super::produce::LodSceneCullRegion;

/// Enqueue culls for hosts overlapping each [`LodSceneCullRegion<M>`].
///
/// Uses a single viewer [`LodRef`] (no per-host dominant level vote). Requires
/// [`LodNestedRefreshAllowed`] + [`LodHostHasCullableRoots`].
pub fn produce_lod_cull_for_region<I, M, T, F>(
	mut commands: Commands,
	mut cull_writer: MessageWriter<LodCullRequest>,
	mut regions: MessageReader<LodSceneCullRegion<M>>,
	index: StaticSystemParam<I>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	hosts: Query<
		(&T, &LodSceneLevel, &Children),
		(
			With<LodSceneHost>,
			With<LodNestedRefreshAllowed>,
			With<LodHostHasCullableRoots>,
		),
	>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	pending: Query<(), With<LodLevelRootPending>>,
	wants_cull: Query<(), With<LodCullInFlight>>,
) where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneRegionIndex<T>,
	M: Send + Sync + 'static,
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	let mut region_iter = regions.read().peekable();
	if region_iter.peek().is_none() {
		return;
	}

	let snapshots = collect_node_snapshots(&nodes);
	if snapshots.is_empty() {
		return;
	}
	let refs = lod_refs_from_snapshots(&snapshots);
	let Some(viewer_ref) = refs.first() else {
		return;
	};

	let mut index = index.into_inner();
	for region_msg in region_iter {
		for (entity, _scene) in index.hosts_in_region(region_msg.region) {
			let Ok((scene, current, host_children)) = hosts.get(entity) else {
				continue;
			};

			let culls = scene.scene_lod_culls(viewer_ref, *current);
			if matches!(culls, LodSceneCulls::None) {
				continue;
			}

			let mut roots_entity = None;
			for child in host_children.iter() {
				if level_roots_heads.contains(child) {
					roots_entity = Some(child);
					break;
				}
			}
			let Some(roots_entity) = roots_entity else {
				continue;
			};
			let Ok(root_children) = level_roots_heads.get(roots_entity) else {
				continue;
			};

			if root_children
				.iter()
				.any(|child| pending.contains(child) && !wants_cull.contains(child))
			{
				continue;
			}

			for child in root_children.iter() {
				let Ok(root) = root_keys.get(child) else {
					continue;
				};
				if root.0 == *current {
					continue;
				}
				if wants_cull.contains(child) {
					continue;
				}
				if culls.should_cull(root.0) {
					enqueue_lod_cull(&mut commands, &mut cull_writer, child, &wants_cull);
				}
			}
		}
	}
}

/// Region channel `M` + index `I` → cull enqueue for hosts `T` (no full-world scan).
pub struct LodSceneRegionCullPlugin<I, M, T, F = With<LodViewer>>
where
	I: SystemParam + 'static,
	M: Send + Sync + 'static,
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (I, M, T, F)>,
}

impl<I, M, T, F> Default for LodSceneRegionCullPlugin<I, M, T, F>
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

impl<I, M, T, F> Plugin for LodSceneRegionCullPlugin<I, M, T, F>
where
	I: SystemParam + 'static,
	M: Send + Sync + 'static,
	T: Component + LodScene + 'static,
	F: QueryFilter + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneRegionIndex<T>,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		if !app.is_plugin_added::<LodChunkBudgetPlugin>() {
			app.add_plugins(LodChunkBudgetPlugin);
		}
		if !app.is_plugin_added::<LodCullMarkerPlugin>() {
			app.add_plugins(LodCullMarkerPlugin);
		}
		app.add_message::<LodSceneCullRegion<M>>().add_systems(
			Update,
			produce_lod_cull_for_region::<I, M, T, F>.in_set(LodChunkCullSystems::Enqueue),
		);
	}
}
