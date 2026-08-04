//! Shared furniture placement helpers for usage-area fills.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use richmond_building_components::Placement;

/// Placement that fills `aabb` with a unit cube centered in the volume.
pub fn placement_filling_aabb(aabb: &Aabb3d) -> Placement {
	let center = Vec3::from((aabb.min + aabb.max) * 0.5);
	let extent = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	Placement::new(center, 0.0).with_scale(extent)
}
