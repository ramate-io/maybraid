//! Avian-backed region index for LOD refresh ([`LodSceneRegionIndex`](lod::LodSceneRegionIndex)).
//!
//! Hosts must carry an Avian [`Collider`] on the **host** entity (no `RigidBody`
//! required — query-only) and a `T: Component + LodScene` on a
//! [`LodSceneHost`](lod::LodSceneHost). Prefer
//! [`PatchSceneBounds`](lod::PatchSceneBounds) with
//! [`AvianLodSceneBoundsMarshaller`] to stamp volumes from
//! [`LodScene::scene_bounds`](lod::LodScene::scene_bounds).

use std::collections::HashSet;
use std::marker::PhantomData;

use avian3d::prelude::{Collider, ColliderAabb, SpatialQuery};
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::LodScene;
use lod::{
	LodSceneBoundsMarshaller, LodSceneHost, LodSceneRegionIndex, LodSceneRefreshPlugin,
	LodViewer, PatchSceneBounds,
};

/// [`LodSceneBoundsMarshaller`] that inserts a query-only Avian [`Collider`] on the host.
#[derive(Debug, Clone, Copy, Default)]
pub struct AvianLodSceneBoundsMarshaller;

impl LodSceneBoundsMarshaller for AvianLodSceneBoundsMarshaller {
	type Volume = Collider;

	fn volume_from_bounds(bounds: Aabb3d) -> Self::Volume {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let center = (min + max) * 0.5;
		let size = (max - min).max(Vec3::splat(1e-3));
		let cuboid = Collider::cuboid(size.x, size.y, size.z);
		if center.length_squared() <= 1e-8 {
			cuboid
		} else {
			Collider::compound(vec![(center, Quat::IDENTITY, cuboid)])
		}
	}
}

/// [`SystemParam`] Avian implementation of [`LodSceneRegionIndex`] for host type `T`.
///
/// Only colliders on the host entity itself count (physics children do not enroll
/// the host in LOD refresh).
#[derive(SystemParam)]
pub struct AvianLodSceneRegionIndex<'w, 's, T: Component + LodScene + 'static> {
	spatial: SpatialQuery<'w, 's>,
	hosts: Query<'w, 's, (Entity, &'static T), With<LodSceneHost>>,
}

impl<T: Component + LodScene + 'static> LodSceneRegionIndex<T>
	for AvianLodSceneRegionIndex<'_, '_, T>
{
	fn hosts_in_region<'a>(&'a self, region: Aabb3d) -> impl Iterator<Item = (Entity, &'a T)> + 'a {
		let collider = ColliderAabb::from_min_max(Vec3::from(region.min), Vec3::from(region.max));
		let hit: HashSet<Entity> =
			self.spatial.aabb_intersections_with_aabb(collider).into_iter().collect();
		self.hosts
			.iter()
			.filter(move |(entity, _)| hit.contains(entity))
			.map(|(entity, scene)| (entity, scene))
	}
}

/// [`LodSceneRefreshPlugin`] with [`AvianLodSceneRegionIndex`] + host volume patch.
///
/// `T` listens for [`lod::LodSceneRefreshRegion`] on channel `M`; levels use
/// [`LodNode`]s filtered by `F` (default: [`LodViewer`]).
pub struct AvianLodSceneRefreshPlugin<T, M, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, M, F)>,
}

impl<T, M, F> Default for AvianLodSceneRefreshPlugin<T, M, F>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<T, M, F> Plugin for AvianLodSceneRefreshPlugin<T, M, F>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PatchSceneBounds<T, AvianLodSceneBoundsMarshaller>>() {
			app.add_plugins(PatchSceneBounds::<T, AvianLodSceneBoundsMarshaller>::default());
		}
		app.add_plugins(LodSceneRefreshPlugin::<
			T,
			M,
			AvianLodSceneRegionIndex<'_, '_, T>,
			F,
		>::default());
	}
}
