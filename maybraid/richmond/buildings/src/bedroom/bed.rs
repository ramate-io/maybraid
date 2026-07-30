//! Bed cell: furniture fill for an allocated sleep volume.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::furniture::FurnitureNode;

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

impl LodScene for Bed {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		_level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		self.furniture.scene_with_lod(lod_ref)
	}
}
