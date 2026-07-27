//! A floor of the Wizard's Tower.
//!
//! The floor cell is rectangular. Inside it, the author:
//! - places circular outer-wall partition components (angular sweeps),
//! - places up to four radial subdividing walls toward the spire radius,
//! - subsets [`CellConstraints`](crate::CellConstraints) for the spire rectangle
//!   and surrounding voxel halfspaces / rooms.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::doors::RoughStoneDoorFrame15;
use richmond_building_components::floors::{RoughStoneFloorArcFill, RoughStoneFloorStructFill};
use richmond_building_components::partitions::rough_stonework::{
	RoughStonework180, RoughStonework90, RoughStoneworkLinear,
};

use crate::wizards_tower::{
	compose_scene, spire_rect, voxel_halfspaces, WizardsTowerRoom, WizardsTowerSpire,
};
use crate::CellConstraints;

/// One storey of the circular tower.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerFloor {
	pub constraints: CellConstraints,
	/// Circular outer wall halves.
	pub outer_walls: [RoughStonework180; 2],
	/// Optional quarter patches / openings kit.
	pub outer_quarters: [RoughStonework90; 4],
	/// Up to four radial subdividers toward the spire.
	pub radial_walls: [RoughStoneworkLinear; 4],
	pub floor_arc: RoughStoneFloorArcFill,
	pub floor_struct: RoughStoneFloorStructFill,
	/// Suggestive exterior door frame on one radial bay.
	pub door_frame: RoughStoneDoorFrame15,
	pub spire: WizardsTowerSpire,
	pub rooms: Vec<WizardsTowerRoom>,
}

impl WizardsTowerFloor {
	/// Build from column parent constraints and this floor's subsetted constraints.
	pub fn new(_parent_constraints: &CellConstraints, constraints: CellConstraints) -> Self {
		let spire_aabb = spire_rect(&constraints.aabb, 0.28);
		let spire_constraints = constraints
			.subset(spire_aabb)
			.unwrap_or_else(|_| CellConstraints::cell_owned(spire_aabb));
		let spire = WizardsTowerSpire::new(&constraints, spire_constraints);

		let rooms = voxel_halfspaces(&constraints.aabb, &spire_aabb)
			.into_iter()
			.filter_map(|room_aabb| {
				// Degenerate halfspaces (zero thickness) are skipped.
				if room_aabb.min.x >= room_aabb.max.x - 1e-5
					|| room_aabb.min.y >= room_aabb.max.y - 1e-5
					|| room_aabb.min.z >= room_aabb.max.z - 1e-5
				{
					return None;
				}
				let room_constraints = constraints
					.subset(room_aabb)
					.unwrap_or_else(|_| CellConstraints::cell_owned(room_aabb));
				Some(WizardsTowerRoom::new(&constraints, room_constraints))
			})
			.collect();

		Self {
			constraints,
			outer_walls: [RoughStonework180, RoughStonework180],
			outer_quarters: [
				RoughStonework90,
				RoughStonework90,
				RoughStonework90,
				RoughStonework90,
			],
			radial_walls: [
				RoughStoneworkLinear,
				RoughStoneworkLinear,
				RoughStoneworkLinear,
				RoughStoneworkLinear,
			],
			floor_arc: RoughStoneFloorArcFill,
			floor_struct: RoughStoneFloorStructFill,
			door_frame: RoughStoneDoorFrame15::default(),
			spire,
			rooms,
		}
	}
}

impl LodScene for WizardsTowerFloor {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		for wall in &self.outer_walls {
			children.push(Box::new(wall.scene_with_lod(lod_ref)));
		}
		for quarter in &self.outer_quarters {
			children.push(Box::new(quarter.scene_with_lod(lod_ref)));
		}
		for radial in &self.radial_walls {
			children.push(Box::new(radial.scene_with_lod(lod_ref)));
		}
		children.push(Box::new(self.floor_arc.scene_with_lod(lod_ref)));
		children.push(Box::new(self.floor_struct.scene_with_lod(lod_ref)));
		children.push(Box::new(self.door_frame.scene_with_lod(lod_ref)));
		children.push(Box::new(self.spire.scene_with_lod(lod_ref)));
		for room in &self.rooms {
			children.push(Box::new(room.scene_with_lod(lod_ref)));
		}
		compose_scene(children)
	}
}
