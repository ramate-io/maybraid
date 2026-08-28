//! Produce [`LodSceneRefreshLevel`] from region impulses and a spatial index.

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
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
use crate::scene::visual::{under_visual_lod_root, VisualLodRoot, VisualOwnsAppearance};
use crate::scene::SemanticLodScene;

use super::super::viewer::LodViewer;
use super::super::{ensure_refresh_core, LodLevelProduceSystems};

/// Impulse: set host `entity` toward `level` (folded by max in entity refresh).
#[derive(Message, Debug, Clone, Copy)]
pub struct LodSceneRefreshLevel {
	pub entity: Entity,
	pub level: LodSceneLevel,
}

/// Untyped refresh AABB (union of every [`LodSceneRefreshRegion<M>`] channel).
///
/// Region production writes this beside the typed channel message. One fill
/// system reads it so produce is once per host type, not once per channel.
#[derive(Message, Debug, Clone, Copy)]
pub struct LodSceneRefreshAabb {
	pub region: Aabb3d,
}

/// This-frame driver snapshots + host hits per unique refresh AABB.
///
/// Filled once ([`fill_lod_produce_cache`]); every `T` reuses it.
#[derive(Resource, Debug, Default)]
pub struct LodProduceCache {
	pub snapshots: Vec<LodNodeSnapshot>,
	pub region_hits: Vec<(Aabb3d, Vec<Entity>)>,
}

impl LodProduceCache {
	fn clear(&mut self) {
		self.snapshots.clear();
		self.region_hits.clear();
	}

	fn has_region(&self, region: Aabb3d) -> bool {
		self.region_hits.iter().any(|(r, _)| *r == region)
	}
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
		if cache.has_region(msg.region) {
			continue;
		}
		let hits: Vec<Entity> = index.hosts_in_region(msg.region).collect();
		cache.region_hits.push((msg.region, hits));
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
	owns_visual: Query<(), With<VisualOwnsAppearance>>,
	visual_roots: Query<(), With<VisualLodRoot>>,
) where
	T: Component + SemanticLodScene + 'static,
{
	if cache.region_hits.is_empty() || cache.snapshots.is_empty() {
		return;
	}
	let refs = lod_refs_from_snapshots(&cache.snapshots);
	for (_region, hits) in &cache.region_hits {
		for &entity in hits {
			if owns_visual.contains(entity)
				|| under_visual_lod_root(entity, &child_of, &visual_roots)
			{
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

/// Emit levels for host `T` from the shared [`LodProduceCache`].
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
		app.add_systems(
			Update,
			produce_lod_refresh_levels::<T>.in_set(LodLevelProduceSystems::Emit),
		);
	}
}
