//! Ensuite bathroom cell: separating panel walls + fixture placeholders.

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::bedroom::shell::{opening_return_rectangle, ShellWall};
use crate::bedroom::{owns_face_as_cell, placement_filling_aabb};
use crate::constraints::FaceKind;
use crate::CellConstraints;

const WALL_THICK: f32 = 0.12;

/// Ensuite volume: walls toward the bedroom + vanity / toilet wireframes.
#[derive(Debug, Clone, PartialEq)]
pub struct EnsuiteBathroom {
	pub constraints: CellConstraints,
	/// Face of [`Self::constraints`] that opens into the bedroom.
	pub open_face: FaceKind,
	pub walls: Vec<ShellWall>,
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

	fn shell_walls(constraints: &CellConstraints, open_face: FaceKind) -> Vec<ShellWall> {
		let mut walls = Vec::new();
		if owns_face_as_cell(constraints, open_face) {
			if let Some(r) = opening_return_rectangle(&constraints.aabb, open_face, WALL_THICK) {
				walls.push(ShellWall(r));
			}
		}
		walls
	}
}

impl BuildingComponents for EnsuiteBathroom {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for w in &self.walls {
			out.extend(w.panel_nodes_for_level(level));
		}
		out
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		Layers::from_free(vec![self.vanity.clone(), self.toilet.clone()])
	}
}
