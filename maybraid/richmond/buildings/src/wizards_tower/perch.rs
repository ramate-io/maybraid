//! Larger top-floor perch capping the Wizard's Tower.

use bevy::scene::prelude::Scene;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::floors::{rough_stone_floor, Floor};
use richmond_building_components::partitions::{rough_stone_wall, Wall};
use richmond_building_components::roofs::{roof_scene, Roof};
use richmond_building_components::{scene_children, Placed};

use crate::wizards_tower::{WizardsTowerRoom, WizardsTowerSpire};
use crate::CellConstraints;

/// Top perch: wider circular platform over the column.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerPerch {
	pub constraints: CellConstraints,
	pub outer_walls: [Placed<Wall>; 2],
	pub radial_walls: [Placed<Wall>; 4],
	pub floor_arc: Placed<Floor>,
	pub floor_struct: Placed<Floor>,
	pub roof: Placed<Roof>,
	pub deck: Placed<Roof>,
	pub spire: WizardsTowerSpire,
	pub rooms: Vec<WizardsTowerRoom>,
}

impl WizardsTowerPerch {
	/// Build from column parent constraints and this perch's subsetted constraints.
	pub fn new(_parent_constraints: &CellConstraints, constraints: CellConstraints) -> Self {
		let spire_aabb = Self::spire_rect(&constraints.aabb, 0.22);
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
		let extent = constraints.aabb.max - constraints.aabb.min;
		let radius = 0.5 * extent.x.min(extent.z);
		let floor_height = extent.y.max(1e-4);
		let ring_scale = Vec3::new(radius, floor_height, radius);

		Self {
			outer_walls: [
				Placed::new(Wall::arc(180.0), center_xz, 0.0).with_scale(ring_scale),
				Placed::new(Wall::arc(180.0), center_xz, std::f32::consts::PI).with_scale(ring_scale),
			],
			radial_walls: [
				Placed::new(Wall::linear(), center_xz, 0.0).with_scale(ring_scale),
				Placed::new(Wall::linear(), center_xz, std::f32::consts::FRAC_PI_2)
					.with_scale(ring_scale),
				Placed::new(Wall::linear(), center_xz, std::f32::consts::PI).with_scale(ring_scale),
				Placed::new(
					Wall::linear(),
					center_xz,
					std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
				)
				.with_scale(ring_scale),
			],
			floor_arc: Placed::new(Floor::arc_fill(360.0), center_xz, 0.0).with_scale(ring_scale),
			floor_struct: Placed::at_origin(Floor::struct_fill()),
			roof: Placed::new(
				Roof::perch(),
				Vec3::new(center.x, constraints.aabb.max.y, center.z),
				0.0,
			)
			.with_scale(ring_scale),
			deck: Placed::new(Roof::deck(), center_xz, 0.0).with_scale(ring_scale),
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

impl LodScene for WizardsTowerPerch {
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
		children.push(Box::new(roof_scene(&self.roof, lod_ref)));
		children.push(Box::new(roof_scene(&self.deck, lod_ref)));
		children.push(Box::new(self.spire.scene_with_lod(lod_ref)));
		for room in &self.rooms {
			children.push(Box::new(room.scene_with_lod(lod_ref)));
		}
		scene_children(children)
	}
}
