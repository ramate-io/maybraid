//! Central circular spire region of a Wizard's Tower floor.
//!
//! Geometry: four 90° core wall arcs only for now. Stairs / roofs / floor fill
//! inside the spire hole are omitted (empty scenes).

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::partitions::{Wall, WallNode};
use richmond_building_components::scene_children;
use richmond_building_components::Placement;

use crate::CellConstraints;

/// Spire cell with exclusive boundary rights in its write bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerSpire {
	pub constraints: CellConstraints,
	pub core_walls: [WallNode; 4],
}

impl WizardsTowerSpire {
	/// Build from this spire's subsetted constraints.
	pub fn new(constraints: CellConstraints) -> Self {
		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);
		let extent = constraints.aabb.max - constraints.aabb.min;
		let radius = 0.5 * extent.x.min(extent.z);
		let height = extent.y.max(1e-4);
		let scale = Vec3::new(radius, height, radius);
		Self {
			core_walls: [
				WallNode::rough_stone(
					Wall::arc(90.0),
					Placement::new(center_xz, 0.0).with_scale(scale),
				),
				WallNode::rough_stone(
					Wall::arc(90.0),
					Placement::new(center_xz, std::f32::consts::FRAC_PI_2).with_scale(scale),
				),
				WallNode::rough_stone(
					Wall::arc(90.0),
					Placement::new(center_xz, std::f32::consts::PI).with_scale(scale),
				),
				WallNode::rough_stone(
					Wall::arc(90.0),
					Placement::new(center_xz, std::f32::consts::PI + std::f32::consts::FRAC_PI_2)
						.with_scale(scale),
				),
			],
			constraints,
		}
	}
}

impl LodScene for WizardsTowerSpire {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.core_walls
			.iter()
			.map(|wall| Box::new(wall.scene_with_lod(lod_ref)) as Box<dyn Scene>)
			.collect();
		scene_children(children)
	}
}
