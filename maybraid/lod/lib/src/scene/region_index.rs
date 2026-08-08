//! Region queries for LOD refresh (hosts overlapping an AABB).
//!
//! Separate from [`crate::gen::SpatialIndex`]: refresh only needs “which
//! [`LodScene`] hosts overlap this AABB?”, not generation/storage/`Id`s.

use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::math::bounding::Aabb3d;

use crate::scene::LodScene;

/// Lookup of [`LodScene`] hosts whose colliders / bounds hit `region`.
///
/// Level production is generic over this; how hosts entered the world (authored,
/// generated, etc.) is out of scope.
pub trait LodSceneRegionIndex<T: Component + LodScene> {
	fn hosts_in_region<'a>(
		&'a self,
		region: Aabb3d,
	) -> impl Iterator<Item = (Entity, &'a T)> + 'a;
}
