//! Larger top-floor perch capping the Wizard's Tower.
//!
//! Structurally similar to a floor, but with a wider exterior ring and roof
//! components rather than another storey above.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::floors::{RoughStoneFloorArcFill, RoughStoneFloorStructFill};
use richmond_building_components::partitions::rough_stonework::{
	RoughStonework180, RoughStoneworkLinear,
};
use richmond_building_components::roofs::{RoughStonePerchRoof, WoodPerchDeck};

use crate::wizards_tower::{
	compose_scene, spire_rect, voxel_halfspaces, WizardsTowerRoom, WizardsTowerSpire,
};
use crate::CellConstraints;

/// Top perch: wider circular platform over the column.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerPerch {
	pub constraints: CellConstraints,
	pub outer_walls: [RoughStonework180; 2],
	pub radial_walls: [RoughStoneworkLinear; 4],
	pub floor_arc: RoughStoneFloorArcFill,
	pub floor_struct: RoughStoneFloorStructFill,
	pub roof: RoughStonePerchRoof,
	pub deck: WoodPerchDeck,
	pub spire: WizardsTowerSpire,
	pub rooms: Vec<WizardsTowerRoom>,
}

impl WizardsTowerPerch {
	/// Build from column parent constraints and this perch's subsetted constraints.
	pub fn new(_parent_constraints: &CellConstraints, constraints: CellConstraints) -> Self {
		let spire_aabb = spire_rect(&constraints.aabb, 0.22);
		let spire_constraints = constraints
			.subset(spire_aabb)
			.unwrap_or_else(|_| CellConstraints::cell_owned(spire_aabb));
		let spire = WizardsTowerSpire::new(&constraints, spire_constraints);

		let rooms = voxel_halfspaces(&constraints.aabb, &spire_aabb)
			.into_iter()
			.filter_map(|room_aabb| {
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
			radial_walls: [
				RoughStoneworkLinear,
				RoughStoneworkLinear,
				RoughStoneworkLinear,
				RoughStoneworkLinear,
			],
			floor_arc: RoughStoneFloorArcFill,
			floor_struct: RoughStoneFloorStructFill,
			roof: RoughStonePerchRoof,
			deck: WoodPerchDeck,
			spire,
			rooms,
		}
	}
}

impl LodScene for WizardsTowerPerch {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		for wall in &self.outer_walls {
			children.push(Box::new(wall.scene_with_lod(lod_ref)));
		}
		for radial in &self.radial_walls {
			children.push(Box::new(radial.scene_with_lod(lod_ref)));
		}
		children.push(Box::new(self.floor_arc.scene_with_lod(lod_ref)));
		children.push(Box::new(self.floor_struct.scene_with_lod(lod_ref)));
		children.push(Box::new(self.roof.scene_with_lod(lod_ref)));
		children.push(Box::new(self.deck.scene_with_lod(lod_ref)));
		children.push(Box::new(self.spire.scene_with_lod(lod_ref)));
		for room in &self.rooms {
			children.push(Box::new(room.scene_with_lod(lod_ref)));
		}
		compose_scene(children)
	}
}
