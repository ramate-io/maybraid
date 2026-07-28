//! Ensuite bathroom cell: separating walls + fixture placeholders.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::partitions::{Wall, WallNode};
use richmond_building_components::scene_children;
use richmond_building_components::Placement;

use crate::bedroom::{owns_face_as_cell, placement_filling_aabb};
use crate::constraints::FaceKind;
use crate::CellConstraints;

/// Ensuite volume: walls toward the bedroom + vanity / toilet wireframes.
#[derive(Debug, Clone, PartialEq)]
pub struct EnsuiteBathroom {
	pub constraints: CellConstraints,
	pub walls: Vec<WallNode>,
	pub vanity: FurnitureNode,
	pub toilet: FurnitureNode,
}

impl EnsuiteBathroom {
	pub fn new(constraints: CellConstraints) -> Self {
		let walls = Self::shell_walls(&constraints);
		let aabb = &constraints.aabb;
		let size = aabb.max - aabb.min;
		let vanity_aabb = bevy_math::bounding::Aabb3d::from_min_max(
			Vec3::new(aabb.min.x + 0.1, aabb.min.y, aabb.min.z + 0.15),
			Vec3::new(
				aabb.min.x + size.x * 0.55,
				aabb.min.y + 0.85,
				aabb.min.z + 0.55,
			),
		);
		let toilet_aabb = bevy_math::bounding::Aabb3d::from_min_max(
			Vec3::new(aabb.max.x - 0.55, aabb.min.y, aabb.max.z - 0.7),
			Vec3::new(aabb.max.x - 0.1, aabb.min.y + 0.75, aabb.max.z - 0.15),
		);
		Self {
			constraints,
			walls,
			vanity: FurnitureNode::vanity(placement_filling_aabb(&vanity_aabb)),
			toilet: FurnitureNode::toilet(placement_filling_aabb(&toilet_aabb)),
		}
	}

	fn shell_walls(constraints: &CellConstraints) -> Vec<WallNode> {
		let aabb = &constraints.aabb;
		let size = aabb.max - aabb.min;
		let y0 = aabb.min.y;
		let h = size.y.max(1e-4);
		let cz = (aabb.min.z + aabb.max.z) * 0.5;
		let half_z = size.z * 0.5;
		let thick = 0.12_f32 / 0.2;

		let mut walls = Vec::new();
		// Room-facing wall (−X / Left). Door gap deferred — full separator for v1.
		if owns_face_as_cell(constraints, FaceKind::Left) {
			walls.push(WallNode::rough_stone(
				Wall::linear(),
				Placement::new(Vec3::new(aabb.min.x, y0, cz), std::f32::consts::FRAC_PI_2)
					.with_scale(Vec3::new(half_z, h, thick)),
			));
		}
		walls
	}
}

impl LodScene for EnsuiteBathroom {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = self
			.walls
			.iter()
			.map(|w| Box::new(w.scene_with_lod(lod_ref)) as Box<dyn Scene>)
			.collect();
		children.push(Box::new(self.vanity.scene_with_lod(lod_ref)));
		children.push(Box::new(self.toilet.scene_with_lod(lod_ref)));
		scene_children(children)
	}
}
