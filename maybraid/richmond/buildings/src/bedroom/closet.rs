//! Closet cell: partition shell + wardrobe fill.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::partitions::{Wall, WallNode};
use richmond_building_components::scene_children;
use richmond_building_components::Placement;

use crate::bedroom::{owns_face_as_cell, placement_filling_aabb};
use crate::constraints::FaceKind;
use crate::CellConstraints;

/// Closet volume: walls facing the room + wardrobe furniture.
#[derive(Debug, Clone, PartialEq)]
pub struct Closet {
	pub constraints: CellConstraints,
	pub walls: Vec<WallNode>,
	pub wardrobe: FurnitureNode,
}

impl Closet {
	pub fn new(constraints: CellConstraints) -> Self {
		let walls = Self::shell_walls(&constraints);
		let wardrobe = FurnitureNode::wardrobe(placement_filling_aabb(&constraints.aabb));
		Self {
			constraints,
			walls,
			wardrobe,
		}
	}

	/// Walls on the closet shell (open toward +Z into the bedroom living area).
	/// Only faces wholly owned by this cell are emitted.
	fn shell_walls(constraints: &CellConstraints) -> Vec<WallNode> {
		let aabb = &constraints.aabb;
		let size = aabb.max - aabb.min;
		let y0 = aabb.min.y;
		let h = size.y.max(1e-4);
		let cx = (aabb.min.x + aabb.max.x) * 0.5;
		let cz = (aabb.min.z + aabb.max.z) * 0.5;
		let half_x = size.x * 0.5;
		let half_z = size.z * 0.5;
		let thick = 0.12_f32 / 0.2;

		let mut walls = Vec::new();
		// Back (−Z / Front face)
		if owns_face_as_cell(constraints, FaceKind::Front) {
			walls.push(WallNode::rough_stone(
				Wall::linear(),
				Placement::new(Vec3::new(cx, y0, aabb.min.z), 0.0)
					.with_scale(Vec3::new(half_x, h, thick)),
			));
		}
		// −X
		if owns_face_as_cell(constraints, FaceKind::Left) {
			walls.push(WallNode::rough_stone(
				Wall::linear(),
				Placement::new(Vec3::new(aabb.min.x, y0, cz), std::f32::consts::FRAC_PI_2)
					.with_scale(Vec3::new(half_z, h, thick)),
			));
		}
		// +X
		if owns_face_as_cell(constraints, FaceKind::Right) {
			walls.push(WallNode::rough_stone(
				Wall::linear(),
				Placement::new(Vec3::new(aabb.max.x, y0, cz), std::f32::consts::FRAC_PI_2)
					.with_scale(Vec3::new(half_z, h, thick)),
			));
		}
		// Front header-ish low wall toward room (+Z / Back) — short return for opening
		if owns_face_as_cell(constraints, FaceKind::Back) {
			walls.push(WallNode::rough_stone(
				Wall::linear(),
				Placement::new(Vec3::new(aabb.min.x + half_x * 0.35, y0, aabb.max.z), 0.0)
					.with_scale(Vec3::new(half_x * 0.35, h, thick)),
			));
		}
		walls
	}
}

impl LodScene for Closet {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = self
			.walls
			.iter()
			.map(|w| Box::new(w.scene_with_lod(lod_ref)) as Box<dyn Scene>)
			.collect();
		children.push(Box::new(self.wardrobe.scene_with_lod(lod_ref)));
		scene_children(children)
	}
}
