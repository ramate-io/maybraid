//! One circular wall storey: two 180° arcs scaled to radius and floor height.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::partitions::{rough_stone_wall, Wall};
use richmond_building_components::{scene_children, Placed};

/// A single ring of outer circular walls.
#[derive(Debug, Clone, PartialEq)]
pub struct StackedRing {
	pub base_y: f32,
	pub floor_height: f32,
	pub radius: f32,
	pub outer_walls: [Placed<Wall>; 2],
}

impl StackedRing {
	/// Place two 180° arcs at `base_y`, scaled to `(radius, floor_height, radius)`.
	pub fn new(base_y: f32, floor_height: f32, radius: f32) -> Self {
		let translation = Vec3::new(0.0, base_y, 0.0);
		let scale = Vec3::new(radius, floor_height, radius);
		Self {
			base_y,
			floor_height,
			radius,
			outer_walls: [
				Placed::new(Wall::arc(180.0), translation, 0.0).with_scale(scale),
				Placed::new(Wall::arc(180.0), translation, std::f32::consts::PI).with_scale(scale),
			],
		}
	}
}

impl LodScene for StackedRing {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.outer_walls
			.iter()
			.map(|wall| Box::new(rough_stone_wall(wall, lod_ref)) as Box<dyn Scene>)
			.collect();
		scene_children(children)
	}
}
