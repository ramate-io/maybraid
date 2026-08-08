//! Avian-backed region index for LOD refresh ([`LodSceneRegionIndex`](lod::LodSceneRegionIndex)).
//!
//! Hosts must carry an Avian collider (so they appear in [`SpatialQuery`] AABB
//! tests) and a `T: Component + LodScene` on a [`LodSceneHost`](lod::LodSceneHost).

use std::collections::HashSet;
use std::marker::PhantomData;

use avian3d::prelude::{ColliderAabb, SpatialQuery};
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::LodScene;
use lod::{LodSceneHost, LodSceneRegionIndex, LodSceneRefreshPlugin, LodViewer};

/// [`SystemParam`] Avian implementation of [`LodSceneRegionIndex`] for host type `T`.
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
		self.hosts.iter().filter(move |(entity, _)| hit.contains(entity))
	}
}

/// [`LodSceneRefreshPlugin`] with [`AvianLodSceneRegionIndex`].
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
		app.add_plugins(LodSceneRefreshPlugin::<
			T,
			M,
			AvianLodSceneRegionIndex<'_, '_, T>,
			F,
		>::default());
	}
}
