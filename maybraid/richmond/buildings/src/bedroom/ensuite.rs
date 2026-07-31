//! Ensuite bathroom cell: separating walls + fixture placeholders.

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::partitions::{
	wall_placement_from_centered, Partition, PartitionNode,
};
use richmond_building_components::{BuildingComponents, Layers};

use crate::bedroom::{owns_face_as_cell, placement_filling_aabb};
use crate::constraints::FaceKind;
use crate::CellConstraints;

/// Ensuite volume: walls toward the bedroom + vanity / toilet wireframes.
#[derive(Debug, Clone, PartialEq)]
pub struct EnsuiteBathroom {
	pub constraints: CellConstraints,
	/// Face of [`Self::constraints`] that opens into the bedroom.
	pub open_face: FaceKind,
	pub walls: Vec<PartitionNode>,
	pub vanity: FurnitureNode,
	pub toilet: FurnitureNode,
}

impl EnsuiteBathroom {
	pub fn new(constraints: CellConstraints, open_face: FaceKind) -> Self {
		let walls = Self::shell_walls(&constraints, open_face);
		let aabb = &constraints.aabb;
		let size = aabb.max - aabb.min;
		let vanity_aabb = bevy_math::bounding::Aabb3d::from_min_max(
			Vec3::new(aabb.min.x + 0.1, aabb.min.y, aabb.min.z + 0.15),
			Vec3::new(aabb.min.x + size.x * 0.55, aabb.min.y + 0.85, aabb.min.z + 0.55),
		);
		let toilet_aabb = bevy_math::bounding::Aabb3d::from_min_max(
			Vec3::new(aabb.max.x - 0.55, aabb.min.y, aabb.max.z - 0.7),
			Vec3::new(aabb.max.x - 0.1, aabb.min.y + 0.75, aabb.max.z - 0.15),
		);
		Self {
			constraints,
			open_face,
			walls,
			vanity: FurnitureNode::vanity(placement_filling_aabb(&vanity_aabb)),
			toilet: FurnitureNode::toilet(placement_filling_aabb(&toilet_aabb)),
		}
	}

	fn shell_walls(constraints: &CellConstraints, open_face: FaceKind) -> Vec<PartitionNode> {
		let aabb = &constraints.aabb;
		let size = aabb.max - aabb.min;
		let y0 = aabb.min.y;
		let h = size.y.max(1e-4);
		let half_z = size.z * 0.5;
		let thick = 0.12_f32 / 0.2;

		let mut walls = Vec::new();
		// Room-facing separator on `open_face`, with a door leave (swing already reserved).
		if owns_face_as_cell(constraints, open_face) {
			match open_face {
				FaceKind::Left => {
					walls.push(PartitionNode::rough_stone(
						Partition::linear(),
						wall_placement_from_centered(
							Vec3::new(aabb.min.x, y0, aabb.min.z + half_z * 0.35),
							std::f32::consts::FRAC_PI_2,
							half_z * 0.35,
							h,
							thick,
						),
					));
				}
				FaceKind::Right => {
					walls.push(PartitionNode::rough_stone(
						Partition::linear(),
						wall_placement_from_centered(
							Vec3::new(aabb.max.x, y0, aabb.min.z + half_z * 0.35),
							std::f32::consts::FRAC_PI_2,
							half_z * 0.35,
							h,
							thick,
						),
					));
				}
				FaceKind::Front => {
					let half_x = size.x * 0.5;
					walls.push(PartitionNode::rough_stone(
						Partition::linear(),
						wall_placement_from_centered(
							Vec3::new(aabb.min.x + half_x * 0.35, y0, aabb.min.z),
							0.0,
							half_x * 0.35,
							h,
							thick,
						),
					));
				}
				FaceKind::Back => {
					let half_x = size.x * 0.5;
					walls.push(PartitionNode::rough_stone(
						Partition::linear(),
						wall_placement_from_centered(
							Vec3::new(aabb.min.x + half_x * 0.35, y0, aabb.max.z),
							0.0,
							half_x * 0.35,
							h,
							thick,
						),
					));
				}
				FaceKind::Top | FaceKind::Bottom => {}
			}
		}
		walls
	}
}

impl BuildingComponents for EnsuiteBathroom {
	fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartitionNode> {
		Layers::from_free(self.walls.clone())
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		Layers::from_free(vec![self.vanity.clone(), self.toilet.clone()])
	}
}

