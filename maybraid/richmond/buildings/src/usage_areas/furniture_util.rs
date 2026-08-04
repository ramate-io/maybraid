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

/// Build a labeled furniture fill from a packed AABB.
///
/// Kit reuse: `dresser` ≈ counters/surfaces, `bedroom_furniture` ≈ tables/seating,
/// `nightstand` / `wardrobe` for compact fillers and storage silhouettes.
pub fn furniture_fill(
	style: LabelStyle,
	text: &str,
	aabb: &Aabb3d,
	roll: f32,
	make: fn(Placement) -> FurnitureNode,
) -> FurnitureFill {
	FurnitureFill {
		label: label_filling_aabb(style, text, aabb, roll),
		furniture: make(placement_filling_aabb(aabb)),
	}
}

/// Placement that fills `aabb` with a unit cube centered in the volume.
pub fn placement_filling_aabb(aabb: &Aabb3d) -> Placement {
	let center = Vec3::from((aabb.min + aabb.max) * 0.5);
	let extent = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	Placement::new(center, 0.0).with_scale(extent)
}
