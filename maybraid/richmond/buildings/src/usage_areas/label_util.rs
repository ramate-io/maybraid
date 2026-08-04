//! Shared [`LabelNode`] helpers for usage-area placeholders (AABB → filled label).

use bevy_math::bounding::{Aabb3d, BoundingVolume};
use bevy_math::Vec3;
use richmond_building_components::{LabelNode, LabelStyle};

/// Label filling `aabb` (unit cube scaled to extents).
pub fn label_filling_aabb(style: LabelStyle, text: &str, aabb: &Aabb3d, yaw: f32) -> LabelNode {
	let center = Vec3::from(aabb.center());
	let extents = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	LabelNode::rectangle(style, text, center, extents, yaw)
}
