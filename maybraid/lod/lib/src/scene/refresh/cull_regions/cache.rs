//! Shared cull produce cache: one untyped host-hit query per unique cull AABB.

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::lod_ref::{
	collect_node_snapshots, LodNode, LodNodeBounds, LodNodePose, LodNodeSnapshot,
};
use crate::scene::region_index::LodSceneHostIndex;

use super::super::ensure_refresh_core;
use super::super::sync::LodChunkCullSystems;
use super::super::viewer::LodViewer;

/// Untyped cull AABB (union of every [`super::LodSceneCullRegion<M>`] channel).
///
/// Region production writes this beside the typed channel message. One fill
/// system reads it so the Avian query is once per unique region, not once per `T`.
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
}

impl LodCullProduceCache {
	fn clear(&mut self) {
		self.snapshots.clear();
		self.region_hits.clear();
	}

	fn has_region(&self, region: Aabb3d) -> bool {
		self.region_hits.iter().any(|(r, _)| *r == region)
	}
}

/// Collect driver refs and untyped host hits once per frame.
pub fn fill_lod_cull_produce_cache<I, F>(
	mut regions: MessageReader<LodSceneCullAabb>,
	index: StaticSystemParam<I>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	mut cache: ResMut<LodCullProduceCache>,
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
		if cache.has_region(msg.region) {
			continue;
		}
		let hits: Vec<Entity> = index.hosts_in_region(msg.region).collect();
		cache.region_hits.push((msg.region, hits));
	}
}

/// Fill [`LodCullProduceCache`] from untyped cull AABBs via host index `I`.
pub struct LodSceneCullProduceFillPlugin<I, F = With<LodViewer>>
where
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (I, F)>,
}

impl<I, F> Default for LodSceneCullProduceFillPlugin<I, F>
where
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<I, F> Plugin for LodSceneCullProduceFillPlugin<I, F>
where
	I: SystemParam + 'static,
	F: QueryFilter + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_systems(
			Update,
			fill_lod_cull_produce_cache::<I, F>.in_set(LodChunkCullSystems::FillCache),
		);
	}
}
