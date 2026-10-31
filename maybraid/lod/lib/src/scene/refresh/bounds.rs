//! Host AABBs (scene / spawn concerns — not [`crate::lod_ref::LodRef`] driver extents).

use bevy::prelude::*;
use bevy::shape::Aabb3d;

/// AABB for a [`crate::LodSceneHost`] (host geometry / indexing helpers).
///
/// Driver extents for [`crate::lod_ref::LodRef`] live on [`crate::LodNodeBounds`].
#[derive(Debug, Clone, Copy, Component)]
pub struct LodHostBounds(pub Aabb3d);

impl Default for LodHostBounds {
	fn default() -> Self {
		Self(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE))
	}
}
