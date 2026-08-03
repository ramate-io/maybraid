//! Shared Label helpers for usage-area placeholders.

use bevy_math::bounding::{Aabb3d, BoundingVolume};
use bevy_math::Vec3;
use richmond_building_components::{LabelNode, LabelStyle};

/// Label filling `aabb` (unit cube scaled to extents).
pub fn label_filling_aabb(style: LabelStyle, text: &str, aabb: &Aabb3d, yaw: f32) -> LabelNode {
	let center = Vec3::from(aabb.center());
	let extents = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	LabelNode::rectangle(style, text, center, extents, yaw)
}

/// Shrink `aabb` toward its center by `pad` on XZ (Y unchanged).
pub fn inset_xz(aabb: &Aabb3d, pad: f32) -> Aabb3d {
	let min = Vec3::from(aabb.min);
	let max = Vec3::from(aabb.max);
	let hx = ((max.x - min.x) * 0.5 - pad).max(0.05);
	let hz = ((max.z - min.z) * 0.5 - pad).max(0.05);
	let c = Vec3::from(aabb.center());
	Aabb3d::from_min_max(
		Vec3::new(c.x - hx, min.y, c.z - hz),
		Vec3::new(c.x + hx, max.y, c.z + hz),
	)
}
