//! Region-scoped cull enqueue from the shared [`LodCullProduceCache`].

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

use crate::lod_ref::lod_refs_from_snapshots;
use crate::scene::cull::LodSceneCulls;
use crate::scene::host::{
	lod_level_roots_entity, lod_scene_host_or_ancestor_hidden, LodLevelRoot, LodLevelRoots,
	LodSceneHost,
};
use crate::scene::level::LodSceneLevel;
use crate::scene::region_index::LodSceneHostIndex;
use crate::scene::SemanticLodScene;

use super::super::ensure_refresh_core;
use super::super::sync::{enqueue_lod_cull, LodChunkBudgetPlugin, LodCullInFlight, LodCullRequest};
use super::super::viewer::LodViewer;
use super::super::LodSceneRefreshLevelsPlugin;
use super::cache::{LodCullProduceCache, LodSceneCullProduceFillPlugin};
use super::markers::{LodCullMarkerPlugin, LodNestedRefreshAllowed};
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
	all_hosts: Query<(), With<LodSceneHost>>,
	mut host_levels: Query<&mut LodSceneLevel, With<LodSceneHost>>,
	host_children_q: Query<&Children, With<LodSceneHost>>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	wants_cull: Query<(), With<LodCullInFlight>>,
	child_of: Query<&ChildOf>,
	visibilities: Query<&Visibility>,
) where
	T: Component + SemanticLodScene + 'static,
{
	if cache.region_hits.is_empty() || cache.snapshots.is_empty() {
		return;
	}
	if hosts.is_empty() {
		return;
	}

	let refs = lod_refs_from_snapshots(&cache.snapshots);
	let Some(viewer_ref) = refs.first() else {
		return;
	};

	for (_region, hits) in &cache.region_hits {
		for &entity in hits {
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
			if distance_level < *current {
				*current = distance_level;
			}
			let current_level = *current;
			drop(current);

			let culls = scene.scene_lod_culls(viewer_ref, current_level);
			if matches!(culls, LodSceneCulls::None) {
				continue;
			}

			let Ok(host_children) = host_children_q.get(entity) else {
				continue;
			};
			let Some(roots_entity) = lod_level_roots_entity(host_children, &level_roots_heads)
			else {
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
					enqueue_lod_cull(&mut commands, &mut cull_writer, child, &wants_cull);
				}
			}
		}
	}
}

/// Enqueue region-scoped culls once for all semantic host types.
pub fn produce_lod_cull_for_region_erased(world: &mut World) {
	let (snapshots, hits) = {
		let cache = world.resource::<LodCullProduceCache>();
		if cache.region_hits.is_empty() || cache.snapshots.is_empty() {
			return;
		}
		let hits = cache
			.region_hits
			.iter()
			.flat_map(|(_, entities)| entities.iter().copied())
			.collect::<HashSet<_>>();
		(cache.snapshots.clone(), hits)
	};
	let refs = lod_refs_from_snapshots(&snapshots);
	let Some(viewer_ref) = refs.first() else {
		return;
	};

	for entity in hits {
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
		if distance_level < current_level {
			current_level = distance_level;
			world.entity_mut(entity).insert(current_level);
		}
		let Some(culls) = producer.culls_for(world, entity, viewer_ref, current_level) else {
			continue;
		};
		if matches!(culls, LodSceneCulls::None) {
			continue;
		}

		let roots: Vec<_> = world
			.get::<Children>(entity)
			.and_then(|children| {
				children.iter().find(|child| world.get::<LodLevelRoots>(*child).is_some())
			})
			.and_then(|bag| world.get::<Children>(bag))
			.map(|children| children.iter().collect())
			.unwrap_or_default();
		for root_entity in roots {
			let Some(root) = world.get::<LodLevelRoot>(root_entity).copied() else {
				continue;
			};
			if root.0 == current_level
				|| world.get::<LodCullInFlight>(root_entity).is_some()
				|| !culls.should_cull(root.0)
			{
				continue;
			}
			world
				.entity_mut(root_entity)
				.insert((LodCullInFlight { started: false }, Visibility::Hidden));
			world.write_message(LodCullRequest { entity: root_entity });
		}
	}
}

fn lod_scene_host_or_ancestor_hidden_world(world: &World, entity: Entity) -> bool {
	let mut current = Some(entity);
	while let Some(entity) = current {
		if world.get::<LodSceneHost>(entity).is_some()
			&& world
				.get::<Visibility>(entity)
				.is_some_and(|visibility| matches!(*visibility, Visibility::Hidden))
		{
			return true;
		}
		current = world.get::<ChildOf>(entity).map(|child| child.parent());
	}
	false
}

/// Register `T` for the shared erased enqueue from [`LodCullProduceCache`].
///
/// Channel `M` stays so existing `AvianLodSceneCullPlugin<T, M, F>` adds remain
/// valid; the spatial query is registered once per (`I`, `F`).
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
		if !app.is_plugin_added::<LodSceneCullProduceFillPlugin<I, F>>() {
			app.add_plugins(LodSceneCullProduceFillPlugin::<I, F>::default());
		}
		app.add_message::<LodSceneCullRegion<M>>();
	}
}
