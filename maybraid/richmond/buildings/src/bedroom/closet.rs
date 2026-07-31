//! Closet cell: partition shell + wardrobe fill.

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

/// Closet volume: walls facing the room + wardrobe furniture.
#[derive(Debug, Clone, PartialEq)]
pub struct Closet {
	pub constraints: CellConstraints,
	/// Face of [`Self::constraints`] that opens into the bedroom (door swing outward).
	pub open_face: FaceKind,
	pub walls: Vec<PartitionNode>,
	pub wardrobe: FurnitureNode,
}

impl Closet {
	pub fn new(constraints: CellConstraints, open_face: FaceKind) -> Self {
		let walls = Self::shell_walls(&constraints, open_face);
		let wardrobe = FurnitureNode::wardrobe(placement_filling_aabb(&constraints.aabb));
		Self { constraints, open_face, walls, wardrobe }
	}

	/// Shell walls with a doorway leave on `open_face` (already swing-budgeted by layout).
	fn shell_walls(constraints: &CellConstraints, open_face: FaceKind) -> Vec<PartitionNode> {
		let aabb = &constraints.aabb;
		let size = aabb.max - aabb.min;
		let y0 = aabb.min.y;
		let h = size.y.max(1e-4);
		let cx = (aabb.min.x + aabb.max.x) * 0.5;
		let cz = (aabb.min.z + aabb.max.z) * 0.5;
		let half_x = size.x * 0.5;
		let half_z = size.z * 0.5;
		let thick = 0.12_f32 / 0.2;

		let mut walls = Vec::new();
		for face in [FaceKind::Front, FaceKind::Back, FaceKind::Left, FaceKind::Right] {
			if !owns_face_as_cell(constraints, face) {
				continue;
			}
			if face == open_face {
				// Partial return wall beside the opening (door leave).
				push_opening_return(&mut walls, aabb, face, y0, h, thick);
			} else {
				push_full_face_wall(&mut walls, aabb, face, cx, cz, half_x, half_z, y0, h, thick);
			}
		}
		walls
	}
}

fn push_full_face_wall(
	walls: &mut Vec<PartitionNode>,
	aabb: &bevy_math::bounding::Aabb3d,
	face: FaceKind,
	cx: f32,
	cz: f32,
	half_x: f32,
	half_z: f32,
	y0: f32,
	h: f32,
	thick: f32,
) {
	match face {
		FaceKind::Front => walls.push(PartitionNode::rough_stone(
			Partition::linear(),
			wall_placement_from_centered(Vec3::new(cx, y0, aabb.min.z), 0.0, half_x, h, thick),
		)),
		FaceKind::Back => walls.push(PartitionNode::rough_stone(
			Partition::linear(),
			wall_placement_from_centered(Vec3::new(cx, y0, aabb.max.z), 0.0, half_x, h, thick),
		)),
		FaceKind::Left => walls.push(PartitionNode::rough_stone(
			Partition::linear(),
			wall_placement_from_centered(
				Vec3::new(aabb.min.x, y0, cz),
				std::f32::consts::FRAC_PI_2,
				half_z,
				h,
				thick,
			),
		)),
		FaceKind::Right => walls.push(PartitionNode::rough_stone(
			Partition::linear(),
			wall_placement_from_centered(
				Vec3::new(aabb.max.x, y0, cz),
				std::f32::consts::FRAC_PI_2,
				half_z,
				h,
				thick,
			),
		)),
		FaceKind::Top | FaceKind::Bottom => {}
	}
}

fn push_opening_return(
	walls: &mut Vec<PartitionNode>,
	aabb: &bevy_math::bounding::Aabb3d,
	face: FaceKind,
	y0: f32,
	h: f32,
	thick: f32,
) {
	let size = aabb.max - aabb.min;
	match face {
		FaceKind::Front | FaceKind::Back => {
			let half_x = size.x * 0.5;
			let z = if face == FaceKind::Front { aabb.min.z } else { aabb.max.z };
			// Short return on the −X side of the opening.
			walls.push(PartitionNode::rough_stone(
				Partition::linear(),
				wall_placement_from_centered(
					Vec3::new(aabb.min.x + half_x * 0.35, y0, z),
					0.0,
					half_x * 0.35,
					h,
					thick,
				),
			));
		}
		FaceKind::Left | FaceKind::Right => {
			let half_z = size.z * 0.5;
			let x = if face == FaceKind::Left { aabb.min.x } else { aabb.max.x };
			walls.push(PartitionNode::rough_stone(
				Partition::linear(),
				wall_placement_from_centered(
					Vec3::new(x, y0, aabb.min.z + half_z * 0.35),
					std::f32::consts::FRAC_PI_2,
					half_z * 0.35,
					h,
					thick,
				),
			));
		}
		FaceKind::Top | FaceKind::Bottom => {}
	}
}

impl BuildingComponents for Closet {
	fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartitionNode> {
		Layers::from_free(self.walls.clone())
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		Layers::from_free(vec![self.wardrobe.clone()])
	}
}

