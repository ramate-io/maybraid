//! Larger top-floor perch capping the Wizard's Tower.
//!
//! Same treatment as a regular storey for now: outer rings + squared floor only.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::floors::{rough_stone_floor, Floor};
use richmond_building_components::partitions::{rough_stone_wall, Wall};
use richmond_building_components::{scene_children, Placed};

use crate::wizards_tower::floor_fill::{
	squared_floor_with_spire_hole, SPIRE_HALF_FRAC, WALL_HEIGHT_MULT,
};
use crate::CellConstraints;

/// Top perch: wider circular platform over the column.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerPerch {
	pub constraints: CellConstraints,
	pub outer_walls: [Placed<Wall>; 2],
	pub floor_caps: [Placed<Floor>; 4],
	pub floor_rects: [Placed<Floor>; 4],
}

impl WizardsTowerPerch {
	/// Build from column parent constraints and this perch's subsetted constraints.
	pub fn new(_parent_constraints: &CellConstraints, constraints: CellConstraints) -> Self {
		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);
		let extent = constraints.aabb.max - constraints.aabb.min;
		let radius = 0.5 * extent.x.min(extent.z);
		let floor_height = extent.y.max(1e-4);
		let ring_scale = Vec3::new(radius, floor_height * WALL_HEIGHT_MULT, radius);
		let spire_half = SPIRE_HALF_FRAC * radius;
		let (floor_caps, floor_rects) =
			squared_floor_with_spire_hole(center_xz, radius, spire_half);

		Self {
			outer_walls: [
				Placed::new(Wall::arc(180.0), center_xz, 0.0).with_scale(ring_scale),
				Placed::new(Wall::arc(180.0), center_xz, std::f32::consts::PI).with_scale(ring_scale),
			],
			floor_caps,
			floor_rects,
			constraints,
		}
	}
}

impl LodScene for WizardsTowerPerch {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		for wall in &self.outer_walls {
			children.push(Box::new(rough_stone_wall(wall, lod_ref)));
		}
		for cap in &self.floor_caps {
			children.push(Box::new(rough_stone_floor(cap, lod_ref)));
		}
		for rect in &self.floor_rects {
			children.push(Box::new(rough_stone_floor(rect, lod_ref)));
		}
		scene_children(children)
	}
}
