//! Host AABBs for ephemeral [`crate::scene::LodRef`] construction.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

/// AABB for a [`crate::LodSceneHost`] used when building ephemeral [`crate::scene::LodRef`]s.
#[derive(Debug, Clone, Copy, Component)]
pub struct LodHostBounds(pub Aabb3d);

impl Default for LodHostBounds {
	fn default() -> Self {
		Self(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE))
	}
}

/// Placeholder bounds when a host has no [`LodHostBounds`] (probe-driven hosts).
pub(crate) fn ephemeral_bounds(host_bounds: Option<&LodHostBounds>) -> Aabb3d {
	host_bounds
		.map(|b| b.0)
		.unwrap_or_else(|| Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE))
}
