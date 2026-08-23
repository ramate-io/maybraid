//! Region queries for LOD refresh (hosts overlapping an AABB).
//!
//! Separate from [`crate::gen::SpatialIndex`]: refresh only needs “which
//! [`LodScene`] hosts overlap this AABB?”, not generation/storage/`Id`s.

use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::math::bounding::Aabb3d;

use crate::scene::LodScene;

/// Lookup of **any** [`crate::LodSceneHost`] whose host volume hits `region`.
///
/// Level production fills a frame cache from this, then each host type `T`
/// filters with `Query<&T>`. Cull still uses [`LodSceneRegionIndex`].
pub trait LodSceneHostIndex {
	fn hosts_in_region<'a>(&'a mut self, region: Aabb3d) -> impl Iterator<Item = Entity> + 'a;
}

/// Lookup of [`LodScene`] hosts of type `T` whose **host** volumes hit `region`.
///
/// Used by region-scoped cull. Level production uses [`LodSceneHostIndex`] so
/// the Avian AABB query runs once per unique region, not once per `T`.
///
/// How hosts entered the world (authored, generated, etc.) is out of scope.
/// Volumes are typically stamped by [`crate::PatchSceneBounds`] from
/// [`crate::LodScene::scene_bounds`].
pub trait LodSceneRegionIndex<T: Component + LodScene> {
	fn hosts_in_region<'a>(
		&'a mut self,
		region: Aabb3d,
	) -> impl Iterator<Item = (Entity, &'a T)> + 'a;
}
