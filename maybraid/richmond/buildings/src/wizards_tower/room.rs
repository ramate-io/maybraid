//! Voxel halfspace / room fill around the Wizard's Tower spire.
//!
//! Parents pass subsetted [`CellConstraints`](crate::CellConstraints). The room
//! fills geometry within those constraints without claiming the spire rectangle.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::doors::{RoughStoneDoorFrame15, WoodDoorLeaf};
use richmond_building_components::floors::{WoodFloorArcFill, WoodFloorStructFill};
use richmond_building_components::partitions::rough_stonework::RoughStoneworkLinear;
use richmond_building_components::stairs::WoodStraightStair;

use crate::wizards_tower::compose_scene;
use crate::CellConstraints;

/// A bounded room / voxel-halfspace child of a tower floor.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerRoom {
	pub constraints: CellConstraints,
	pub partition: RoughStoneworkLinear,
	pub floor_arc: WoodFloorArcFill,
	pub floor_struct: WoodFloorStructFill,
	pub door_frame: RoughStoneDoorFrame15,
	pub door_leaf: WoodDoorLeaf,
	pub stair: WoodStraightStair,
}

impl WizardsTowerRoom {
	/// Build from floor/perch parent constraints and this room's subsetted constraints.
	pub fn new(_parent_constraints: &CellConstraints, constraints: CellConstraints) -> Self {
		Self {
			constraints,
			partition: RoughStoneworkLinear,
			floor_arc: WoodFloorArcFill,
			floor_struct: WoodFloorStructFill,
			door_frame: RoughStoneDoorFrame15::default(),
			door_leaf: WoodDoorLeaf,
			stair: WoodStraightStair,
		}
	}
}

impl LodScene for WizardsTowerRoom {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		compose_scene(vec![
			Box::new(self.partition.scene_with_lod(lod_ref)),
			Box::new(self.floor_arc.scene_with_lod(lod_ref)),
			Box::new(self.floor_struct.scene_with_lod(lod_ref)),
			Box::new(self.door_frame.scene_with_lod(lod_ref)),
			Box::new(self.door_leaf.scene_with_lod(lod_ref)),
			Box::new(self.stair.scene_with_lod(lod_ref)),
		])
	}
}
