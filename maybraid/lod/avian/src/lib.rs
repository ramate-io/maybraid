//! Avian-backed region index for LOD refresh ([`LodSceneRegionIndex`](lod::LodSceneRegionIndex)).
//!
//! Hosts must carry an Avian collider (so they appear in [`SpatialQuery`] AABB
//! tests) and a `T: Component + LodScene` on a [`LodSceneHost`](lod::LodSceneHost).

use std::collections::HashSet;

use avian3d::prelude::{ColliderAabb, SpatialQuery};
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::LodScene;
use lod::{add_lod_scene_refresh_for, LodSceneHost, LodSceneRegionIndex};

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

/// Register [`add_lod_scene_refresh_for`] with [`AvianLodSceneRegionIndex`].
///
/// Still requires [`lod::LodFinePassPlugin`] for viewer track + root sync.
pub fn add_avian_lod_scene_refresh_for<T: Component + LodScene + 'static>(app: &mut App) {
	add_lod_scene_refresh_for::<T, AvianLodSceneRegionIndex<'_, '_, T>>(app);
}
