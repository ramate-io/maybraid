//! Produce [`LodSceneRefreshLevel`] from region impulses and a spatial index.

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose,
	LodNodeSnapshot,
};
use crate::scene::host::{
	nested_host_parent_allows_refresh, LodLevelRoot, LodLevelRoots, LodSceneHost,
};
use crate::scene::level::LodSceneLevel;
use crate::scene::region_index::LodSceneHostIndex;
use crate::scene::visual::{under_visual_lod_root, VisualLodRoot};
use crate::scene::SemanticLodScene;

use super::super::viewer::LodViewer;
use super::super::{ensure_refresh_core, LodLevelProduceSystems};

/// Impulse: set host `entity` toward `level` (folded by max in entity refresh).
#[derive(Message, Debug, Clone, Copy)]
pub struct LodSceneRefreshLevel {
	pub entity: Entity,
	pub level: LodSceneLevel,
}

type ProduceLevelFn =
	for<'a> fn(&World, Entity, &[crate::lod_ref::LodRef<'a>]) -> Option<LodSceneLevel>;

/// Type-erased level callback stamped once when a semantic host component is added.
///
/// This lets the shared producer visit each spatial hit once instead of running
/// one hit scan for every registered host type.
#[derive(Component, Clone, Copy)]
pub struct LodLevelProducer(ProduceLevelFn);

impl LodLevelProducer {
	fn for_scene<T>() -> Self
	where
		T: Component + SemanticLodScene + 'static,
	{
		Self(|world, entity, refs| {
			world.get::<T>(entity).map(|scene| scene.scene_lod_level_from_levels(refs))
		})
	}
}

fn attach_lod_level_producer<T>(add: On<Add, T>, mut commands: Commands)
where
	T: Component + SemanticLodScene + 'static,
{
	if let Ok(mut entity) = commands.get_entity(add.entity) {
		entity.insert(LodLevelProducer::for_scene::<T>());
	}
}

/// Untyped refresh AABB (union of every [`LodSceneRefreshRegion<M>`] channel).
///
/// Region production writes this beside the typed channel message. One fill
/// system reads it so produce is once per host type, not once per channel.
#[derive(Message, Debug, Clone, Copy)]
pub struct LodSceneRefreshAabb {
	pub region: Aabb3d,
}

/// This-frame driver snapshots + deduplicated host hits.
///
/// Filled once ([`fill_lod_produce_cache`]); every `T` reuses it.
#[derive(Resource, Debug, Default)]
pub struct LodProduceCache {
	pub snapshots: Vec<LodNodeSnapshot>,
	/// Union of all unique refresh-region hits.
	///
	/// The shared erased producer visits each entity once, independent of the
	/// number of registered semantic host types.
	pub hit_entities: HashSet<Entity>,
	regions: Vec<Aabb3d>,
}

impl LodProduceCache {
	fn clear(&mut self) {
		self.snapshots.clear();
		self.hit_entities.clear();
		self.regions.clear();
	}

	fn has_region(&self, region: Aabb3d) -> bool {
		self.regions.contains(&region)
	}

	fn remove_contained_regions(&mut self) {
		self.regions.sort_by(|a, b| region_volume(*b).total_cmp(&region_volume(*a)));
		let mut index = 0;
		while index < self.regions.len() {
			let region = self.regions[index];
			if self.regions[..index].iter().any(|outer| contains_region(*outer, region)) {
				self.regions.remove(index);
			} else {
				index += 1;
			}
		}
	}
}

fn region_volume(region: Aabb3d) -> f32 {
	let size = Vec3::from(region.max - region.min);
	size.x.max(0.0) * size.y.max(0.0) * size.z.max(0.0)
}

fn contains_region(outer: Aabb3d, inner: Aabb3d) -> bool {
	outer.min.cmple(inner.min).all() && outer.max.cmpge(inner.max).all()
}

/// Collect driver refs and untyped host hits once per frame.
pub fn fill_lod_produce_cache<I, F>(
	mut regions: MessageReader<LodSceneRefreshAabb>,
	index: StaticSystemParam<I>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	mut cache: ResMut<LodProduceCache>,
) where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
	F: QueryFilter + 'static,
{
	cache.clear();
	if regions.is_empty() {
		return;
	}
	cache.snapshots = collect_node_snapshots(&nodes);
	if cache.snapshots.is_empty() {
		return;
	}

	let mut index = index.into_inner();
	for msg in regions.read() {
		if !cache.has_region(msg.region) {
			cache.regions.push(msg.region);
		}
	}
	cache.remove_contained_regions();
	for region_index in 0..cache.regions.len() {
		let region = cache.regions[region_index];
		cache.hit_entities.extend(index.hosts_in_region(region));
	}
}

/// Emit [`LodSceneRefreshLevel`] for hosts `T` overlapping this frame's regions.
pub fn produce_lod_refresh_levels<T>(
	cache: Res<LodProduceCache>,
	hosts: Query<&T, With<LodSceneHost>>,
	mut levels: MessageWriter<LodSceneRefreshLevel>,
	child_of: Query<&ChildOf>,
	host_levels: Query<&LodSceneLevel, With<LodSceneHost>>,
	level_roots: Query<&LodLevelRoot>,
	children_q: Query<&Children>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	visibilities: Query<&Visibility>,
	visual_roots: Query<(), With<VisualLodRoot>>,
) where
	T: Component + SemanticLodScene + 'static,
{
	if cache.hit_entities.is_empty() || cache.snapshots.is_empty() {
		return;
	}
	let refs = lod_refs_from_snapshots(&cache.snapshots);
	for &entity in &cache.hit_entities {
		if under_visual_lod_root(entity, &child_of, &visual_roots) {
			continue;
		}
		let Ok(scene) = hosts.get(entity) else {
			continue;
		};
		if !nested_host_parent_allows_refresh(
			entity,
			&child_of,
			&host_levels,
			&level_roots,
			&children_q,
			&level_roots_bags,
			&visibilities,
		) {
			continue;
		}
		let level = scene.scene_lod_level_from_levels(&refs);
		levels.write(LodSceneRefreshLevel { entity, level });
	}
}

/// Emit levels once from the shared spatial-hit cache.
pub fn produce_lod_refresh_levels_erased(world: &mut World) {
	let (hits, snapshots) = {
		let cache = world.resource::<LodProduceCache>();
		if cache.hit_entities.is_empty() || cache.snapshots.is_empty() {
			return;
		}
		(cache.hit_entities.iter().copied().collect::<Vec<_>>(), cache.snapshots.clone())
	};
	let refs = lod_refs_from_snapshots(&snapshots);
	let mut produced = Vec::with_capacity(hits.len());
	for entity in hits {
		if under_visual_lod_root_world(world, entity)
			|| !nested_host_parent_allows_refresh_world(world, entity)
		{
			continue;
		}
		let Some(producer) = world.get::<LodLevelProducer>(entity).copied() else {
			continue;
		};
		let Some(level) = (producer.0)(world, entity, &refs) else {
			continue;
		};
		produced.push(LodSceneRefreshLevel { entity, level });
	}
	world.write_message_batch(produced);
}

fn under_visual_lod_root_world(world: &World, entity: Entity) -> bool {
	let mut current = entity;
	loop {
		if world.get::<VisualLodRoot>(current).is_some() {
			return true;
		}
		let Some(parent) = world.get::<ChildOf>(current) else {
			return false;
		};
		current = parent.parent();
	}
}

fn nested_host_parent_allows_refresh_world(world: &World, entity: Entity) -> bool {
	let Some(parent) = world.get::<ChildOf>(entity) else {
		return true;
	};
	let mut current = parent.parent();
	let mut enclosing_root = None;
	loop {
		if enclosing_root.is_none() {
			enclosing_root = world.get::<LodLevelRoot>(current).map(|root| root.0);
		}
		if world.get::<LodSceneHost>(current).is_some() {
			if let Some(desired) = world.get::<LodSceneLevel>(current) {
				return enclosing_root.is_none_or(|root_level| {
					root_level == *desired
						|| host_shows_level_root_world(world, current, root_level)
				});
			}
		}
		let Some(parent) = world.get::<ChildOf>(current) else {
			return true;
		};
		current = parent.parent();
	}
}

fn host_shows_level_root_world(world: &World, host: Entity, level: LodSceneLevel) -> bool {
	let Some(host_children) = world.get::<Children>(host) else {
		return false;
	};
	let Some(bag) = host_children.iter().find(|&child| world.get::<LodLevelRoots>(child).is_some())
	else {
		return false;
	};
	let Some(root_children) = world.get::<Children>(bag) else {
		return false;
	};
	root_children.iter().any(|root| {
		world.get::<LodLevelRoot>(root).is_some_and(|key| key.0 == level)
			&& world
				.get::<Visibility>(root)
				.is_some_and(|visibility| !matches!(*visibility, Visibility::Hidden))
	})
}

/// Fill [`LodProduceCache`] from untyped region AABBs via host index `I`.
pub struct LodSceneRefreshLevelsFillPlugin<I, F = With<LodViewer>>
where
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (I, F)>,
}

impl<I, F> Default for LodSceneRefreshLevelsFillPlugin<I, F>
where
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<I, F> Plugin for LodSceneRefreshLevelsFillPlugin<I, F>
where
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_systems(
			Update,
			fill_lod_produce_cache::<I, F>.in_set(LodLevelProduceSystems::FillCache),
		);
	}
}

/// Register host `T` with the shared erased level producer.
pub struct LodSceneRefreshLevelsPlugin<T>
where
	T: Component + SemanticLodScene + 'static,
{
	_marker: PhantomData<fn() -> T>,
}

impl<T> Default for LodSceneRefreshLevelsPlugin<T>
where
	T: Component + SemanticLodScene + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T> Plugin for LodSceneRefreshLevelsPlugin<T>
where
	T: Component + SemanticLodScene + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_observer(attach_lod_level_producer::<T>);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn contained_refresh_region_is_removed_before_spatial_query() {
		let outer = Aabb3d::from_min_max(Vec3::splat(-100.0), Vec3::splat(100.0));
		let inner = Aabb3d::from_min_max(Vec3::splat(-10.0), Vec3::splat(10.0));
		let mut cache = LodProduceCache { regions: vec![inner, outer], ..Default::default() };
		cache.remove_contained_regions();
		assert_eq!(cache.regions, vec![outer]);
	}
}
