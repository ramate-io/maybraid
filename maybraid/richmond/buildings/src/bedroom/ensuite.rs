//! Ensuite bathroom cell: separating panel walls (fixtures filled later).

use lod::gen::LodSceneLevel;
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::bedroom::shell::{opening_return_rectangle, ShellWall};
use crate::bedroom::owns_face_as_cell;
use crate::constraints::FaceKind;
use crate::CellConstraints;

/// Wall thickness scale (world \(0.12\) / kit \(Y\) half-extent \(0.2\)).
const WALL_THICK: f32 = 0.12 / 0.2;

/// Ensuite volume: walls toward the bedroom; interior left to a residual fill.
#[derive(Debug, Clone, PartialEq)]
pub struct EnsuiteBathroom {
	pub constraints: CellConstraints,
	/// Face of [`Self::constraints`] that opens into the bedroom.
	pub open_face: FaceKind,
	pub walls: Vec<ShellWall>,
}

impl EnsuiteBathroom {
	pub fn new(constraints: CellConstraints, open_face: FaceKind) -> Self {
		let walls = Self::shell_walls(&constraints, open_face);
		Self {
			constraints,
			open_face,
			walls,
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
		Layers::new()
	}
}
