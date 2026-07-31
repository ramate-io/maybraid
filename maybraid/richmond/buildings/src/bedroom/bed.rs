//! Bed cell: furniture fill for an allocated sleep volume.

use lod::gen::LodSceneLevel;
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::bedroom::placement_filling_aabb;
use crate::CellConstraints;

/// Bed volume filled by a placeholder furniture node.
#[derive(Debug, Clone, PartialEq)]
pub struct Bed {
	pub constraints: CellConstraints,
	pub furniture: FurnitureNode,
}

impl Bed {
	pub fn new(constraints: CellConstraints) -> Self {
		let furniture = FurnitureNode::bed(placement_filling_aabb(&constraints.aabb));
		Self { constraints, furniture }
	}
}

impl BuildingComponents for Bed {
	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		Layers::from_free(vec![self.furniture.clone()])
	}
}

