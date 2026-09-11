//! Shared cull produce cache: one untyped host-hit query per unique cull AABB.

use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

use crate::lod_ref::{
	collect_node_snapshots, LodNode, LodNodeBounds, LodNodePose, LodNodeSnapshot,
};
use crate::scene::region_index::LodSceneHostIndex;

use super::super::ensure_refresh_core;
use super::super::sync::LodChunkCullSystems;

/// Untyped cull AABB (union of every [`super::LodSceneCullRegion<M>`] channel).
///
/// Region production writes this beside the typed channel message. One fill
/// system reads it so the host-index query is once per unique region, not once per `T`.
#[derive(Message, Debug, Clone, Copy)]
pub struct LodSceneCullAabb {
	pub region: Aabb3d,
}

/// This-frame driver snapshots + host hits per unique cull AABB.
///
/// Filled once ([`fill_lod_cull_produce_cache`]); every `T` reuses it.
#[derive(Resource, Debug, Default)]
pub struct LodCullProduceCache {
	pub snapshots: Vec<LodNodeSnapshot>,
	pub region_hits: Vec<(Aabb3d, Vec<Entity>)>,
	/// Deduplicated union consumed by the erased producer.
	pub hit_entities: HashSet<Entity>,
}

impl LodCullProduceCache {
	fn clear(&mut self) {
		self.snapshots.clear();
		self.region_hits.clear();
		self.hit_entities.clear();
	}

	fn has_region(&self, region: Aabb3d) -> bool {
		self.region_hits.iter().any(|(r, _)| *r == region)
	}
}

/// Collect every [`LodNode`] snapshot once, then query hosts per unique cull AABB.
///
/// Region production still filters drivers (`With<Camera>` vs [`super::super::LodViewer`]).
/// Fill must not: those filters used to instantiate two systems that each cleared
/// [`LodCullProduceCache`], so the second walk wiped the first.
pub fn fill_lod_cull_produce_cache<I>(
	mut regions: MessageReader<LodSceneCullAabb>,
	index: StaticSystemParam<I>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), With<LodNode>>,
	mut cache: ResMut<LodCullProduceCache>,
) where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
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
		if cache.has_region(msg.region) {
			continue;
		}
		let hits: Vec<Entity> = index.hosts_in_region(msg.region).collect();
		cache.hit_entities.extend(hits.iter().copied());
		cache.region_hits.push((msg.region, hits));
	}
}

/// Fill [`LodCullProduceCache`] from untyped cull AABBs via host index `I`.
pub struct LodSceneCullProduceFillPlugin<I>
where
	I: SystemParam + 'static,
{
	_marker: PhantomData<fn() -> I>,
}

impl<I> Default for LodSceneCullProduceFillPlugin<I>
where
	I: SystemParam + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<I> Plugin for LodSceneCullProduceFillPlugin<I>
where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_systems(
			Update,
			fill_lod_cull_produce_cache::<I>.in_set(LodChunkCullSystems::FillCache),
		);
	}
}
