//! Central circular spire region of a Wizard's Tower floor.
//!
//! Geometry: four 90° core wall arcs, structural floor fill, spiral stair, and
//! a spire roof. Exclusive boundary rights inside the subsetted write AABB.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::floors::{rough_stone_floor, Floor};
use richmond_building_components::partitions::{rough_stone_wall, Wall};
use richmond_building_components::roofs::{roof_scene, Roof};
use richmond_building_components::stairs::{rough_stone_stair, Stair};
use richmond_building_components::{scene_children, Placed};

use crate::CellConstraints;

/// Spire cell with exclusive boundary rights in its write bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerSpire {
	pub constraints: CellConstraints,
	pub core_walls: [Placed<Wall>; 4],
	pub struct_fill: Placed<Floor>,
	pub spiral: Placed<Stair>,
	pub roof: Placed<Roof>,
}

impl WizardsTowerSpire {
	/// Build from floor/perch parent constraints and this spire's subsetted constraints.
	pub fn new(_parent_constraints: &CellConstraints, constraints: CellConstraints) -> Self {
		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);
		let extent = constraints.aabb.max - constraints.aabb.min;
		let radius = 0.5 * extent.x.min(extent.z);
		let height = extent.y.max(1e-4);
		let scale = Vec3::new(radius, height, radius);
		Self {
			core_walls: [
				Placed::new(Wall::arc(90.0), center_xz, 0.0).with_scale(scale),
				Placed::new(Wall::arc(90.0), center_xz, std::f32::consts::FRAC_PI_2).with_scale(scale),
				Placed::new(Wall::arc(90.0), center_xz, std::f32::consts::PI).with_scale(scale),
				Placed::new(
					Wall::arc(90.0),
					center_xz,
					std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
				)
				.with_scale(scale),
			],
			struct_fill: Placed::at_origin(Floor::struct_fill()),
			spiral: Placed::new(Stair::spiral(), center_xz, 0.0).with_scale(scale),
			roof: Placed::new(
				Roof::spire(),
				Vec3::new(center.x, constraints.aabb.max.y, center.z),
				0.0,
			)
			.with_scale(scale),
			constraints,
		}
	}
}

impl LodScene for WizardsTowerSpire {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = self
			.core_walls
			.iter()
			.map(|wall| Box::new(rough_stone_wall(wall, lod_ref)) as Box<dyn Scene>)
			.collect();
		children.push(Box::new(rough_stone_floor(&self.struct_fill, lod_ref)));
		children.push(Box::new(rough_stone_stair(&self.spiral, lod_ref)));
		children.push(Box::new(roof_scene(&self.roof, lod_ref)));
		scene_children(children)
	}
}
