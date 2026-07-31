//! Closet cell: paneling shell + wardrobe fill.

use lod::gen::LodSceneLevel;
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::bedroom::shell::{face_rectangle, opening_return_rectangle, ShellWall};
use crate::bedroom::{owns_face_as_cell, placement_filling_aabb};
use crate::constraints::FaceKind;
use crate::CellConstraints;

/// Wall thickness scale (world \(0.12\) / kit \(Y\) half-extent \(0.2\)).
const WALL_THICK: f32 = 0.12 / 0.2;

/// Closet volume: walls facing the room + wardrobe furniture.
#[derive(Debug, Clone, PartialEq)]
pub struct Closet {
	pub constraints: CellConstraints,
	/// Face of [`Self::constraints`] that opens into the bedroom (door swing outward).
	pub open_face: FaceKind,
	pub walls: Vec<ShellWall>,
	pub wardrobe: FurnitureNode,
}

impl Closet {
	pub fn new(constraints: CellConstraints, open_face: FaceKind) -> Self {
		let walls = Self::shell_walls(&constraints, open_face);
		let wardrobe = FurnitureNode::wardrobe(placement_filling_aabb(&constraints.aabb));
		Self { constraints, open_face, walls, wardrobe }
	}

	/// Shell walls with a doorway leave on `open_face` (already swing-budgeted by layout).
	fn shell_walls(constraints: &CellConstraints, open_face: FaceKind) -> Vec<ShellWall> {
		let aabb = &constraints.aabb;
		let mut walls = Vec::new();
		for face in [FaceKind::Front, FaceKind::Back, FaceKind::Left, FaceKind::Right] {
			if !owns_face_as_cell(constraints, face) {
				continue;
			}
			let rect = if face == open_face {
				opening_return_rectangle(aabb, face, WALL_THICK)
			} else {
				face_rectangle(aabb, face, WALL_THICK)
			};
			if let Some(r) = rect {
				walls.push(ShellWall(r));
			}
		}
		walls
	}
}

impl BuildingComponents for Closet {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for w in &self.walls {
			out.extend(w.panel_nodes_for_level(level));
		}
		out
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		Layers::from_free(vec![self.wardrobe.clone()])
	}
}
