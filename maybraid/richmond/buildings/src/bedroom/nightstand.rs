//! Nightstand cell: furniture fill beside the bed.

use lod::gen::LodSceneLevel;
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::{BuildingComponents};

use crate::bedroom::placement_filling_aabb;
use crate::CellConstraints;

/// Nightstand volume filled by a placeholder furniture node.
#[derive(Debug, Clone, PartialEq)]
pub struct Nightstand {
	pub constraints: CellConstraints,
	pub furniture: FurnitureNode,
}

impl Nightstand {
	pub fn new(constraints: CellConstraints) -> Self {
		let furniture = FurnitureNode::nightstand(placement_filling_aabb(&constraints.aabb));
		Self { constraints, furniture }
	}
}

impl BuildingComponents for Nightstand {
	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Vec<FurnitureNode> {
		vec![self.furniture.clone()]
	}
}

