//! Produce [`LodSceneRefreshLevel`] from region impulses and a spatial index.

use std::any::TypeId;
use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::platform::collections::{HashMap, HashSet};
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
use crate::scene::{LodSceneCulls, SceneChunk, SemanticLodScene};

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
type ProduceOneLevelFn =
	for<'a> fn(&World, Entity, &crate::lod_ref::LodRef<'a>) -> Option<LodSceneLevel>;
type ProduceCullsFn =
	for<'a> fn(&World, Entity, &crate::lod_ref::LodRef<'a>, LodSceneLevel) -> Option<LodSceneCulls>;
type ProduceChunkFn =
	for<'a> fn(&World, Entity, &crate::lod_ref::LodRef<'a>, LodSceneLevel) -> Option<SceneChunk>;

/// Type-erased level callback stamped when a semantic host component is added.
///
/// The shared producer uses this to visit each spatial hit once, independent of
/// the number of registered host types.
#[derive(Component, Clone, Copy)]
pub struct LodLevelProducer {
	level_from_all: ProduceLevelFn,
	level: ProduceOneLevelFn,
	culls: ProduceCullsFn,
	chunk: ProduceChunkFn,
}

impl LodLevelProducer {
	fn for_scene<T>() -> Self
	where
		T: Component + SemanticLodScene + 'static,
	{
		Self {
			level_from_all: |world, entity, refs| {
				world.get::<T>(entity).map(|scene| scene.scene_lod_level_from_levels(refs))
			},
			level: |world, entity, lod_ref| {
				world.get::<T>(entity).map(|scene| scene.scene_lod_level(lod_ref))
			},
			culls: |world, entity, lod_ref, current| {
				world.get::<T>(entity).map(|scene| scene.scene_lod_culls(lod_ref, current))
			},
			chunk: |world, entity, lod_ref, level| {
				world
					.get::<T>(entity)
					.map(|scene| scene.scene_chunks_with_level(lod_ref, level))
			},
		}
	}

	pub(crate) fn level_for(
		self,
		world: &World,
		entity: Entity,
		lod_ref: &crate::lod_ref::LodRef,
	) -> Option<LodSceneLevel> {
		(self.level)(world, entity, lod_ref)
	}

	pub(crate) fn culls_for(
		self,
		world: &World,
		entity: Entity,
		lod_ref: &crate::lod_ref::LodRef,
		current: LodSceneLevel,
	) -> Option<LodSceneCulls> {
		(self.culls)(world, entity, lod_ref, current)
	}

	pub(crate) fn chunk_for(
		self,
		world: &World,
		entity: Entity,
		lod_ref: &crate::lod_ref::LodRef,
		level: LodSceneLevel,
	) -> Option<SceneChunk> {
		(self.chunk)(world, entity, lod_ref, level)
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

/// Typed channel membership. Dual bullseye + spotlight adds stamp two of these.
#[derive(Component, Debug, Clone, Copy)]
pub struct LodRefreshChannel<M: Send + Sync + 'static> {
	_marker: PhantomData<fn() -> M>,
}

impl<M: Send + Sync + 'static> Default for LodRefreshChannel<M> {
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

/// `TypeId<M>` bag so fill can filter Avian hits without a system per channel.
#[derive(Component, Debug, Clone, Default)]
pub struct LodRefreshChannels {
	ids: HashSet<TypeId>,
}

impl LodRefreshChannels {
	pub fn contains(&self, id: TypeId) -> bool {
		self.ids.contains(&id)
	}

	pub fn insert(&mut self, id: TypeId) {
		self.ids.insert(id);
	}
}

fn attach_lod_refresh_channel<T, M>(add: On<Add, T>, mut commands: Commands)
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
{
	if TypeId::of::<M>() == TypeId::of::<()>() {
		return;
	}
	let entity = add.entity;
	let id = TypeId::of::<M>();
	commands.queue(move |world: &mut World| {
		let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
			return;
		};
		entity_mut.insert(LodRefreshChannel::<M>::default());
		let mut bag = entity_mut.get::<LodRefreshChannels>().cloned().unwrap_or_default();
		bag.insert(id);
		entity_mut.insert(bag);
	});
}

/// This-frame refresh AABBs keyed by `TypeId<M>`.
///
/// Slice C ([#795](https://github.com/ramate-io/maybraid/issues/795)): writers
/// push their own channel. Fill does not union vegetation with mob High.
#[derive(Resource, Debug, Default)]
pub struct LodProduceRegionSink {
	channels: HashMap<TypeId, Vec<Aabb3d>>,
}

impl LodProduceRegionSink {
	pub fn push<M: Send + Sync + 'static>(&mut self, region: Aabb3d) {
		self.channels.entry(TypeId::of::<M>()).or_default().push(region);
	}

	fn take(&mut self) -> HashMap<TypeId, Vec<Aabb3d>> {
		std::mem::take(&mut self.channels)
	}

	pub fn is_empty(&self) -> bool {
		self.channels.values().all(Vec::is_empty)
	}
}

/// Camera or [`LodViewer`] drivers. One fill query — not one plugin per `F`.
pub type LodProduceDriver = Or<(With<Camera>, With<LodViewer>)>;

/// Per-channel regions + host hits after contained-region drop and membership filter.
#[derive(Debug, Default, Clone)]
pub struct LodProduceChannel {
	pub regions: Vec<Aabb3d>,
	pub hit_entities: HashSet<Entity>,
}

impl LodProduceChannel {
	fn has_region(&self, region: Aabb3d) -> bool {
		self.regions.contains(&region)
	}

	fn remove_contained_regions(&mut self) {
		remove_contained_regions(&mut self.regions);
	}
}

/// This-frame driver snapshots + per-`M` host hits.
///
/// Filled once by [`fill_lod_produce_cache`], then consumed once by the erased
/// producer.
#[derive(Resource, Debug, Default)]
pub struct LodProduceCaches {
	pub snapshots: Vec<LodNodeSnapshot>,
	pub channels: HashMap<TypeId, LodProduceChannel>,
}

/// Compatibility alias for [`LodProduceCaches`].
pub type LodProduceCache = LodProduceCaches;

impl LodProduceCaches {
	fn clear(&mut self) {
		self.snapshots.clear();
		self.channels.clear();
	}

	fn is_empty(&self) -> bool {
		self.snapshots.is_empty()
			|| self.channels.values().all(|channel| channel.hit_entities.is_empty())
	}
}

fn remove_contained_regions(regions: &mut Vec<Aabb3d>) {
	regions.sort_by(|a, b| region_volume(*b).total_cmp(&region_volume(*a)));
	let mut index = 0;
	while index < regions.len() {
		let region = regions[index];
		if regions[..index].iter().any(|outer| contains_region(*outer, region)) {
			regions.remove(index);
		} else {
			index += 1;
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

/// Collect driver refs and per-channel host hits once per frame.
pub fn fill_lod_produce_cache<I>(
	mut sink: ResMut<LodProduceRegionSink>,
	index: StaticSystemParam<I>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, LodProduceDriver)>,
	membership: Query<&LodRefreshChannels>,
	mut cache: ResMut<LodProduceCaches>,
) where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	cache.clear();
	let batches = sink.take();
	if batches.is_empty() {
		return;
	}
	cache.snapshots = collect_node_snapshots(&nodes);
	if cache.snapshots.is_empty() {
		return;
	}

	let mut index = index.into_inner();
	for (type_id, regions) in batches {
		let mut channel = LodProduceChannel::default();
		for region in regions {
			if !channel.has_region(region) {
				channel.regions.push(region);
			}
		}
		channel.remove_contained_regions();
		for region_index in 0..channel.regions.len() {
			let region = channel.regions[region_index];
			for entity in index.hosts_in_region(region) {
				if membership.get(entity).is_ok_and(|bag| bag.contains(type_id)) {
					channel.hit_entities.insert(entity);
				}
			}
		}
		if !channel.regions.is_empty() {
			cache.channels.insert(type_id, channel);
		}
	}
}

/// Emit [`LodSceneRefreshLevel`] for hosts `T` overlapping this frame's regions.
pub fn produce_lod_refresh_levels<T>(
	cache: Res<LodProduceCaches>,
	hosts: Query<&T, With<LodSceneHost>>,
	mut levels: MessageWriter<LodSceneRefreshLevel>,
	child_of: Query<&ChildOf>,
	host_levels: Query<&LodSceneLevel, With<LodSceneHost>>,
	level_roots: Query<&LodLevelRoot>,
	children_q: Query<&Children>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	visibilities: Query<&Visibility>,
) where
	T: Component + SemanticLodScene + 'static,
{
	if cache.is_empty() {
		return;
	}
	let refs = lod_refs_from_snapshots(&cache.snapshots);
	for channel in cache.channels.values() {
		for &entity in &channel.hit_entities {
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
}

/// Emit levels from each channel's spatial-hit set.
pub fn produce_lod_refresh_levels_erased(world: &mut World) {
	world.resource_scope(|world, cache: Mut<LodProduceCaches>| {
		if cache.is_empty() {
			return;
		}
		let refs = lod_refs_from_snapshots(&cache.snapshots);
		for channel in cache.channels.values() {
			for &entity in &channel.hit_entities {
				if !nested_host_parent_allows_refresh_world(world, entity) {
					continue;
				}
				let Some(producer) = world.get::<LodLevelProducer>(entity).copied() else {
					continue;
				};
				let Some(level) = (producer.level_from_all)(world, entity, &refs) else {
					continue;
				};
				world.write_message(LodSceneRefreshLevel { entity, level });
			}
		}
	});
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

/// Fill [`LodProduceCaches`] once via host index `I` (camera + [`LodViewer`] drivers).
pub struct LodSceneRefreshLevelsFillPlugin<I>
where
	I: SystemParam + 'static,
{
	_marker: PhantomData<fn() -> I>,
}

impl<I> Default for LodSceneRefreshLevelsFillPlugin<I>
where
	I: SystemParam + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<I> Plugin for LodSceneRefreshLevelsFillPlugin<I>
where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_systems(
			Update,
			fill_lod_produce_cache::<I>.in_set(LodLevelProduceSystems::FillCache),
		);
	}
}

/// Register host `T` on refresh channel `M` (`M = ()` stamps the producer only).
pub struct LodSceneRefreshLevelsPlugin<T, M = ()>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
{
	_marker: PhantomData<fn() -> (T, M)>,
}

impl<T, M> Default for LodSceneRefreshLevelsPlugin<T, M>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T, M> Plugin for LodSceneRefreshLevelsPlugin<T, M>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_observer(attach_lod_level_producer::<T>);
		if TypeId::of::<M>() != TypeId::of::<()>() {
			app.add_observer(attach_lod_refresh_channel::<T, M>);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn contained_refresh_region_is_removed_before_spatial_query() {
		let outer = Aabb3d::from_min_max(Vec3::splat(-100.0), Vec3::splat(100.0));
		let inner = Aabb3d::from_min_max(Vec3::splat(-10.0), Vec3::splat(10.0));
		let mut regions = vec![inner, outer];
		remove_contained_regions(&mut regions);
		assert_eq!(regions, vec![outer]);
	}

	#[test]
	fn contained_drop_is_per_channel() {
		let urban = Aabb3d::from_min_max(Vec3::splat(-200.0), Vec3::splat(200.0));
		let veg = Aabb3d::from_min_max(Vec3::splat(-100.0), Vec3::splat(100.0));
		let mut urban_regions = vec![urban];
		let mut veg_regions = vec![veg];
		remove_contained_regions(&mut urban_regions);
		remove_contained_regions(&mut veg_regions);
		assert_eq!(urban_regions, vec![urban]);
		assert_eq!(veg_regions, vec![veg]);
	}
}
