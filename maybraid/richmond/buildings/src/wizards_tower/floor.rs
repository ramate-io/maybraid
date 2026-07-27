//! A floor of the Wizard's Tower.
//!
//! Geometry: circular outer walls (two 180° arcs), four radial linear subdividers
//! toward the spire, stone arc + struct floor fill, and one door frame on a bay.
//! Children receive subsetted [`CellConstraints`] for the spire rect and voxel rooms.

use bevy::prelude::Children;
use bevy::scene::prelude::{bsn, Scene};
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::doors::{door_scene, Door};
use richmond_building_components::floors::{rough_stone_floor, Floor};
use richmond_building_components::partitions::{rough_stone_wall, Wall};
use richmond_building_components::Placed;

use crate::wizards_tower::{WizardsTowerRoom, WizardsTowerSpire};
use crate::CellConstraints;

/// One storey of the circular tower.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerFloor {
	pub constraints: CellConstraints,
	/// Circular outer wall halves (cell-local).
	pub outer_walls: [Placed<Wall>; 2],
	/// Radial subdividers toward the spire (yaw spaced by π/2).
	pub radial_walls: [Placed<Wall>; 4],
	pub floor_arc: Placed<Floor>,
	pub floor_struct: Placed<Floor>,
	pub door: Placed<Door>,
	pub spire: WizardsTowerSpire,
	pub rooms: Vec<WizardsTowerRoom>,
}

impl WizardsTowerFloor {
	/// Build from column parent constraints and this floor's subsetted constraints.
	pub fn new(_parent_constraints: &CellConstraints, constraints: CellConstraints) -> Self {
		let spire_aabb = Self::spire_rect(&constraints.aabb, 0.28);
		let spire_constraints = constraints
			.subset(spire_aabb)
			.unwrap_or_else(|_| CellConstraints::cell_owned(spire_aabb));
		let spire = WizardsTowerSpire::new(&constraints, spire_constraints);

		let rooms = Self::voxel_halfspaces(&constraints.aabb, &spire_aabb)
			.into_iter()
			.filter_map(|room_aabb| {
				if Self::is_degenerate(&room_aabb) {
					return None;
				}
				let room_constraints = constraints
					.subset(room_aabb)
					.unwrap_or_else(|_| CellConstraints::cell_owned(room_aabb));
				Some(WizardsTowerRoom::new(&constraints, room_constraints))
			})
			.collect();

		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);

		Self {
			outer_walls: [
				Placed::new(Wall::arc(180.0), center_xz, 0.0),
				Placed::new(Wall::arc(180.0), center_xz, std::f32::consts::PI),
			],
			radial_walls: [
				Placed::new(Wall::linear(), center_xz, 0.0),
				Placed::new(Wall::linear(), center_xz, std::f32::consts::FRAC_PI_2),
				Placed::new(Wall::linear(), center_xz, std::f32::consts::PI),
				Placed::new(
					Wall::linear(),
					center_xz,
					std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
				),
			],
			floor_arc: Placed::new(Floor::arc_fill(360.0), center_xz, 0.0),
			floor_struct: Placed::at_origin(Floor::struct_fill()),
			door: Placed::new(
				Door::frame_15(),
				center_xz + Vec3::new(0.0, 0.0, constraints.aabb.min.z - center.z),
				0.0,
			),
			spire,
			rooms,
			constraints,
		}
	}

	fn spire_rect(floor: &Aabb3d, half_extent_frac: f32) -> Aabb3d {
		let center = (floor.min + floor.max) * 0.5;
		let half = (floor.max - floor.min) * 0.5 * half_extent_frac;
		Aabb3d::from_min_max(
			Vec3::new(center.x - half.x, floor.min.y, center.z - half.z),
			Vec3::new(center.x + half.x, floor.max.y, center.z + half.z),
		)
	}

	fn voxel_halfspaces(floor: &Aabb3d, spire: &Aabb3d) -> [Aabb3d; 4] {
		[
			Aabb3d::from_min_max(
				Vec3::new(spire.min.x, floor.min.y, floor.min.z),
				Vec3::new(spire.max.x, floor.max.y, spire.min.z),
			),
			Aabb3d::from_min_max(
				Vec3::new(spire.max.x, floor.min.y, spire.min.z),
				Vec3::new(floor.max.x, floor.max.y, spire.max.z),
			),
			Aabb3d::from_min_max(
				Vec3::new(spire.min.x, floor.min.y, spire.max.z),
				Vec3::new(spire.max.x, floor.max.y, floor.max.z),
			),
			Aabb3d::from_min_max(
				Vec3::new(floor.min.x, floor.min.y, spire.min.z),
				Vec3::new(spire.min.x, floor.max.y, spire.max.z),
			),
		]
	}

	fn is_degenerate(aabb: &Aabb3d) -> bool {
		aabb.min.x >= aabb.max.x - 1e-5
			|| aabb.min.y >= aabb.max.y - 1e-5
			|| aabb.min.z >= aabb.max.z - 1e-5
	}
}

impl LodScene for WizardsTowerFloor {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		for wall in &self.outer_walls {
			children.push(Box::new(rough_stone_wall(wall, lod_ref)));
		}
		for wall in &self.radial_walls {
			children.push(Box::new(rough_stone_wall(wall, lod_ref)));
		}
		children.push(Box::new(rough_stone_floor(&self.floor_arc, lod_ref)));
		children.push(Box::new(rough_stone_floor(&self.floor_struct, lod_ref)));
		children.push(Box::new(door_scene(&self.door, lod_ref)));
		children.push(Box::new(self.spire.scene_with_lod(lod_ref)));
		for room in &self.rooms {
			children.push(Box::new(room.scene_with_lod(lod_ref)));
		}
		bsn! {
			Children [ {children} ]
		}
	}
}
