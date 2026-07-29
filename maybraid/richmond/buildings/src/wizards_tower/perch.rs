//! Larger top-floor perch capping the Wizard's Tower.
//!
//! Same treatment as a regular storey for now: crate-level [`crate::ArcWall`] + squared floor.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;
use richmond_building_components::floors::FloorNode;
use richmond_building_components::scene_children;

use crate::walling::{ArcWall, ArcWallParams};
use crate::wizards_tower::floor_fill::{squared_floor_with_spire_hole, SPIRE_HALF_FRAC};
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

	pub(crate) fn emit_external_features(
		&self,
		children: &mut Vec<Box<dyn Scene>>,
		lod_ref: &LodRef,
	) {
		for wall in &self.arc_wall.partitions {
			children.push(Box::new(wall.scene_with_lod(lod_ref)));
		}
	}

	pub(crate) fn emit_internal_features(
		&self,
		children: &mut Vec<Box<dyn Scene>>,
		lod_ref: &LodRef,
	) {
		use richmond_building_components::ParentConfines;

		let confines = ParentConfines::internal(
			self.storey_confine_center(),
			self.storey_confine_radius(),
		);
		for cap in &self.floor_caps {
			children.push(Box::new(
				cap.clone()
					.with_confines(confines)
					.scene_with_lod(lod_ref),
			));
		}
		for rect in &self.floor_rects {
			children.push(Box::new(
				rect.clone()
					.with_confines(confines)
					.scene_with_lod(lod_ref),
			));
		}
	}

	fn storey_confine_center(&self) -> Vec3 {
		let aabb = &self.constraints.aabb;
		Vec3::from((aabb.min + aabb.max) * 0.5)
	}

	fn storey_confine_radius(&self) -> f32 {
		let aabb = &self.constraints.aabb;
		let extent = aabb.max - aabb.min;
		(0.5 * extent.x.min(extent.z)).max(1e-4)
	}
}

impl LodScene for WizardsTowerPerch {
	fn scene_lod_status(
		&self,
		_lod_ref: &LodRef,
	) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

		fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		_level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		self.emit_external_features(&mut children, lod_ref);
		self.emit_internal_features(&mut children, lod_ref);
		scene_children(children)
	}
}
