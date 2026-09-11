//! Region-scoped cull enqueue from the shared [`LodCullProduceCache`].

use std::marker::PhantomData;

use bevy::ecs::entity_disabling::Disabled;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::lod_ref::lod_refs_from_snapshots;
use crate::scene::cull::LodSceneCulls;
use crate::scene::host::{
	hide_lod_tree_world, lod_level_roots_entity, lod_scene_host_or_ancestor_hidden,
	lod_scene_host_or_ancestor_hidden_world, LodLevelRoot, LodLevelRoots, LodSceneHost,
};
use crate::scene::level::LodSceneLevel;
use crate::scene::region_index::LodSceneHostIndex;
use crate::scene::SemanticLodScene;

use super::super::ensure_refresh_core;
use super::super::sync::{enqueue_lod_cull, LodChunkBudgetPlugin, LodCullInFlight, LodCullRequest};
use super::super::viewer::LodViewer;
use super::super::LodSceneRefreshLevelsPlugin;
use super::cache::{LodCullProduceCache, LodSceneCullProduceFillPlugin};
use super::markers::{LodCullMarkerPlugin, LodHostHasCullableRoots, LodNestedRefreshAllowed};
use super::produce::LodSceneCullRegion;
use crate::scene::LodLevelProducer;

/// Enqueue culls for hosts overlapping this frame's cull AABBs.
///
/// Reuses [`LodCullProduceCache`] (one untyped spatial query). Lowers a stale
/// desired [`LodSceneLevel`] when distance wants a farther band, then GC's
/// non-desired roots per [`LodScene::scene_lod_culls`]. Requires
/// [`LodNestedRefreshAllowed`]. Skips types with no hosts.
pub fn produce_lod_cull_for_region<T>(
	mut commands: Commands,
	mut cull_writer: MessageWriter<LodCullRequest>,
	cache: Res<LodCullProduceCache>,
	hosts: Query<&T, (With<LodSceneHost>, With<LodNestedRefreshAllowed>)>,
	all_hosts: Query<(), (With<LodSceneHost>, Allow<Disabled>)>,
	cullable: Query<(), With<LodHostHasCullableRoots>>,
	mut host_levels: Query<&mut LodSceneLevel, With<LodSceneHost>>,
	host_children_q: Query<&Children, (With<LodSceneHost>, Allow<Disabled>)>,
	level_roots_heads: Query<&Children, (With<LodLevelRoots>, Allow<Disabled>)>,
	root_keys: Query<&LodLevelRoot, Allow<Disabled>>,
	wants_cull: Query<(), (With<LodCullInFlight>, Allow<Disabled>)>,
	pending: Query<(), (With<crate::LodLevelRootPending>, Allow<Disabled>)>,
	child_of: Query<&ChildOf, Allow<Disabled>>,
	visibilities: Query<(&Visibility, Has<Disabled>), Allow<Disabled>>,
) where
	T: Component + SemanticLodScene + 'static,
{
	if cache.hit_entities.is_empty() || cache.snapshots.is_empty() {
		return;
	}
	if hosts.is_empty() {
		return;
	}

	let refs = lod_refs_from_snapshots(&cache.snapshots);
	let Some(viewer_ref) = refs.first() else {
		return;
	};

	for &entity in &cache.hit_entities {
		if lod_scene_host_or_ancestor_hidden(entity, &child_of, &all_hosts, &visibilities) {
			continue;
		}
		let Ok(scene) = hosts.get(entity) else {
			continue;
		};

		let Ok(mut current) = host_levels.get_mut(entity) else {
			continue;
		};
		let distance_level = scene.scene_lod_level(viewer_ref);
		let lowered = distance_level < *current;
		if lowered {
			*current = distance_level;
		}
		let current_level = *current;
		drop(current);
		if !lowered && !cullable.contains(entity) {
			continue;
		}

		let culls = scene.scene_lod_culls(viewer_ref, current_level);
		if matches!(culls, LodSceneCulls::None) {
			continue;
		}

		let Ok(host_children) = host_children_q.get(entity) else {
			continue;
		};
		let Some(roots_entity) = lod_level_roots_entity(host_children, &level_roots_heads) else {
			continue;
		};
		let Ok(root_children) = level_roots_heads.get(roots_entity) else {
			continue;
		};

		for child in root_children.iter() {
			let Ok(root) = root_keys.get(child) else {
				continue;
			};
			if root.0 == current_level {
				continue;
			}
			if wants_cull.contains(child) {
				continue;
			}
			if culls.should_cull(root.0) {
				enqueue_lod_cull(&mut commands, &mut cull_writer, child, &wants_cull, &pending);
			}
		}
	}
}

/// Enqueue region-scoped culls once for all semantic host types.
pub fn produce_lod_cull_for_region_erased(world: &mut World) {
	world.resource_scope(|world, cache: Mut<LodCullProduceCache>| {
		if cache.hit_entities.is_empty() || cache.snapshots.is_empty() {
			return;
		}
		let refs = lod_refs_from_snapshots(&cache.snapshots);
		let Some(viewer_ref) = refs.first() else {
			return;
		};
		let mut roots = Vec::new();

		for &entity in &cache.hit_entities {
			if world.get::<LodNestedRefreshAllowed>(entity).is_none()
				|| lod_scene_host_or_ancestor_hidden_world(world, entity)
			{
				continue;
			}
			let Some(producer) = world.get::<LodLevelProducer>(entity).copied() else {
				continue;
			};
			let Some(mut current_level) = world.get::<LodSceneLevel>(entity).copied() else {
				continue;
			};
			let Some(distance_level) = producer.level_for(world, entity, viewer_ref) else {
				continue;
			};
			let lowered = distance_level < current_level;
			if lowered {
				current_level = distance_level;
				world.entity_mut(entity).insert(current_level);
			}
			if !lowered && world.get::<LodHostHasCullableRoots>(entity).is_none() {
				continue;
			}
			let Some(culls) = producer.culls_for(world, entity, viewer_ref, current_level) else {
				continue;
			};
			if matches!(culls, LodSceneCulls::None) {
				continue;
			}

			roots.clear();
			if let Some(root_children) = world
				.get::<Children>(entity)
				.and_then(|children| {
					children.iter().find(|child| world.get::<LodLevelRoots>(*child).is_some())
				})
				.and_then(|bag| world.get::<Children>(bag))
			{
				roots.extend(root_children.iter());
			}
			for root_entity in roots.iter().copied() {
				let Some(root) = world.get::<LodLevelRoot>(root_entity).copied() else {
					continue;
				};
				if root.0 == current_level
					|| world.get::<LodCullInFlight>(root_entity).is_some()
					|| !culls.should_cull(root.0)
				{
					continue;
				}
				let is_pending = world.get::<crate::LodLevelRootPending>(root_entity).is_some();
				{
					let mut entity = world.entity_mut(root_entity);
					entity.insert(LodCullInFlight { started: false });
					if is_pending {
						entity.insert(Visibility::Hidden);
					} else {
						hide_lod_tree_world(&mut entity);
					}
				}
				world.write_message(LodCullRequest { entity: root_entity });
			}
		}
	});
}

/// Register `T` for the shared erased enqueue from [`LodCullProduceCache`].
///
/// Channel `M` stays so existing `GimmeLodSceneCullPlugin<T, M, F>` adds remain
/// valid; the spatial query is registered once per index `I` (every [`crate::LodNode`]).
/// `F` is accepted so dual Camera / LodViewer plugin adds stay valid; fill does not filter on it.
pub struct LodSceneRegionCullPlugin<I, M, T, F = With<LodViewer>>
where
	I: SystemParam + 'static,
	M: Send + Sync + 'static,
	T: Component + SemanticLodScene + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (I, M, T, F)>,
}

impl<I, M, T, F> Default for LodSceneRegionCullPlugin<I, M, T, F>
where
	I: SystemParam + 'static,
	M: Send + Sync + 'static,
	T: Component + SemanticLodScene + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<I, M, T, F> Plugin for LodSceneRegionCullPlugin<I, M, T, F>
where
	I: SystemParam + 'static,
	M: Send + Sync + 'static,
	T: Component + SemanticLodScene + 'static,
	F: QueryFilter + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		if !app.is_plugin_added::<LodChunkBudgetPlugin>() {
			app.add_plugins(LodChunkBudgetPlugin);
		}
		if !app.is_plugin_added::<LodCullMarkerPlugin>() {
			app.add_plugins(LodCullMarkerPlugin);
		}
		if !app.is_plugin_added::<LodSceneRefreshLevelsPlugin<T>>() {
			app.add_plugins(LodSceneRefreshLevelsPlugin::<T>::default());
		}
		if !app.is_plugin_added::<LodSceneCullProduceFillPlugin<I>>() {
			app.add_plugins(LodSceneCullProduceFillPlugin::<I>::default());
		}
		app.add_message::<LodSceneCullRegion<M>>();
	}
}
