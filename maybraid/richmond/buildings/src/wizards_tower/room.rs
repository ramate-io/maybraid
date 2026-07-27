//! Voxel halfspace / room fill around the Wizard's Tower spire.
//!
//! Geometry: one linear partition on the spire-facing edge, wood rectangle +
//! struct floor fill, a stone door frame with wood leaf, and a wood straight stair.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::doors::{door_scene, Door};
use richmond_building_components::floors::{wood_floor, Floor};
use richmond_building_components::partitions::{rough_stone_wall, Wall};
use richmond_building_components::stairs::{wood_stair, Stair};
use richmond_building_components::{scene_children, Placed};

use crate::CellConstraints;

/// A bounded room / voxel-halfspace child of a tower floor.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerRoom {
	pub constraints: CellConstraints,
	pub partition: Placed<Wall>,
	pub floor: Placed<Floor>,
	pub floor_struct: Placed<Floor>,
	pub door_frame: Placed<Door>,
	pub door_leaf: Placed<Door>,
	pub stair: Placed<Stair>,
}

impl WizardsTowerRoom {
	/// Build from floor/perch parent constraints and this room's subsetted constraints.
	pub fn new(_parent_constraints: &CellConstraints, constraints: CellConstraints) -> Self {
		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);
		let size = constraints.aabb.max - constraints.aabb.min;
		// Face the longer horizontal axis toward the room center for the partition.
		let yaw = if size.x >= size.z {
			std::f32::consts::FRAC_PI_2
		} else {
			0.0
		};

		Self {
			partition: Placed::new(Wall::linear(), center_xz, yaw),
			floor: Placed::new(Floor::rectangle(), center_xz, 0.0),
			floor_struct: Placed::at_origin(Floor::struct_fill()),
			door_frame: Placed::new(Door::frame_15(), center_xz, yaw),
			door_leaf: Placed::new(Door::leaf(), center_xz, yaw),
			stair: Placed::new(Stair::straight(), center_xz, yaw),
			constraints,
		}
	}
}

impl LodScene for WizardsTowerRoom {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let partition = rough_stone_wall(&self.partition, lod_ref);
		let floor = wood_floor(&self.floor, lod_ref);
		let floor_struct = wood_floor(&self.floor_struct, lod_ref);
		let door_frame = door_scene(&self.door_frame, lod_ref);
		let door_leaf = door_scene(&self.door_leaf, lod_ref);
		let stair = wood_stair(&self.stair, lod_ref);
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(partition),
			Box::new(floor),
			Box::new(floor_struct),
			Box::new(door_frame),
			Box::new(door_leaf),
			Box::new(stair),
		];
		scene_children(children)
	}
}
