//! Larger top-floor perch capping the Wizard's Tower.
//!
//! Same treatment as a regular storey for now: crate-level [`crate::ArcWall`] + squared floor.

use bevy::scene::prelude::Scene;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;
use richmond_building_components::floors::FloorNode;
use richmond_building_components::scene_children;

use crate::arc_wall::{ArcWall, ArcWallParams};
use crate::wizards_tower::floor_fill::{squared_floor_with_spire_hole, SPIRE_HALF_FRAC};
use crate::wizards_tower::tower_lod::TowerLodFootprint;
use crate::wizards_tower::must_assign_cardinal_portals;
use crate::CellConstraints;

/// Top perch: wider circular platform over the column.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerPerch {
	pub constraints: CellConstraints,
	/// Storey height in meters (outer ring wall \(Y\) scale).
	pub storey_height: f32,
	pub arc_wall: ArcWall,
	pub floor_caps: [FloorNode; 4],
	pub floor_rects: [FloorNode; 4],
}

impl WizardsTowerPerch {
	/// Build from this perch's subsetted constraints, storey height, and portal noise.
	pub fn new(
		constraints: CellConstraints,
		storey_height: f32,
		portal_noise: NoiseParams,
	) -> Self {
		let storey_height = storey_height.max(1e-4);
		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);
		let extent = constraints.aabb.max - constraints.aabb.min;
		let radius = 0.5 * extent.x.min(extent.z);
		let spire_half = SPIRE_HALF_FRAC * radius;
		let (floor_caps, floor_rects) =
			squared_floor_with_spire_hole(center_xz, radius, spire_half);

		let arc_wall = ArcWall::new(ArcWallParams {
			center_xz,
			radius,
			storey_height,
			arc_degrees: 360.0,
			must_assign: must_assign_cardinal_portals(),
			must_not_assign: vec![],
			portal_noise,
			optional_portals: (0, 2),
		});

		Self {
			storey_height,
			arc_wall,
			floor_caps,
			floor_rects,
			constraints,
		}
	}

	fn emit_external_features(&self, children: &mut Vec<Box<dyn Scene>>, lod_ref: &LodRef) {
		for wall in &self.arc_wall.walls {
			children.push(Box::new(wall.scene_with_lod(lod_ref)));
		}
	}

	fn emit_internal_features(&self, children: &mut Vec<Box<dyn Scene>>, lod_ref: &LodRef) {
		for cap in &self.floor_caps {
			children.push(Box::new(cap.scene_with_lod(lod_ref)));
		}
		for rect in &self.floor_rects {
			children.push(Box::new(rect.scene_with_lod(lod_ref)));
		}
	}
}

impl TowerLodFootprint for WizardsTowerPerch {
	fn lod_aabb(&self) -> &Aabb3d {
		&self.constraints.aabb
	}
}

impl LodScene for WizardsTowerPerch {
	fn scene_lod_status(&self, lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		let prev = self.band_for(lod_ref.previous_transform);
		let curr = self.band_for(lod_ref.current_transform);
		if prev == curr {
			lod::gen::LodSceneStatus::Unchanged
		} else {
			lod::gen::LodSceneStatus::Changed
		}
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		self.emit_external_features(&mut children, lod_ref);
		if self.is_near(lod_ref.current_transform) {
			self.emit_internal_features(&mut children, lod_ref);
		}
		scene_children(children)
	}
}
