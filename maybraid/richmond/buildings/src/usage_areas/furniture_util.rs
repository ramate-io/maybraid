//! Shared furniture placement helpers for usage-area fills.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::placed::Placement;
use richmond_building_components::{LabelNode, LabelStyle};

use crate::usage_areas::label_util::label_filling_aabb;

/// Label + furniture kit pair for one placed AABB.
#[derive(Debug, Clone, PartialEq)]
pub struct FurnitureFill {
	pub label: LabelNode,
	pub furniture: FurnitureNode,
}

/// Build a labeled furniture fill from a packed AABB versus the host volume.
///
/// Stamps abutment, facing yaw, and finish seed from the committed box. Kit
/// reuse: `chair` for seating, `counter` for kitchen runs, `chest` for compact
/// bedroom storage, `dresser` / `wardrobe` for desks and bookcases.
pub fn furniture_fill(
	style: LabelStyle,
	text: &str,
	aabb: &Aabb3d,
	host: &Aabb3d,
	roll: f32,
	make: fn(Placement) -> FurnitureNode,
) -> FurnitureFill {
	let mut furniture = make(placement_filling_aabb(aabb));
	furniture.stamp_host_slot(aabb, host);
	FurnitureFill { label: label_filling_aabb(style, text, aabb, roll), furniture }
}

/// Placement that fills `aabb` with a unit cube centered in the volume.
///
/// Yaw is identity here; [`FurnitureNode::stamp_host_slot`] writes facing.
pub fn placement_filling_aabb(aabb: &Aabb3d) -> Placement {
	let center = Vec3::from((aabb.min + aabb.max) * 0.5);
	let extent = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	Placement::new(center, 0.0).with_scale(extent)
}
