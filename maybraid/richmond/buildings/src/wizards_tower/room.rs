//! Voxel halfspace / room fill around the Wizard's Tower spire.
//!
//! Geometry: one linear partition on the spire-facing edge and a rectangular
//! floor slab. Doors / stairs are omitted for now (empty scenes).

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::floors::{Floor, FloorNode};
use richmond_building_components::partitions::{
	wall_placement_from_centered, Partition, PartitionNode, DEFAULT_THICK,
};
use richmond_building_components::scene_children;
use richmond_building_components::Placement;

use crate::wizards_tower::floor_fill::{FLOOR_SLAB_Y_SCALE, RECT_HALF_EXTENT};
use crate::CellConstraints;

/// A bounded room / voxel-halfspace child of a tower floor.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerRoom {
	pub constraints: CellConstraints,
	pub partition: PartitionNode,
	pub floor: FloorNode,
}

impl WizardsTowerRoom {
	/// Build from this room's subsetted constraints.
	pub fn new(constraints: CellConstraints) -> Self {
		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);
		let size = constraints.aabb.max - constraints.aabb.min;
		let yaw = if size.x >= size.z { std::f32::consts::FRAC_PI_2 } else { 0.0 };
		let half_len = size.x.max(size.z) * 0.5;
		let height = size.y.max(1e-4);
		let floor_scale = Vec3::new(
			size.x.max(1e-4) / (2.0 * RECT_HALF_EXTENT),
			FLOOR_SLAB_Y_SCALE,
			size.z.max(1e-4) / (2.0 * RECT_HALF_EXTENT),
		);

		Self {
			partition: PartitionNode::rough_stone(
				Partition::linear(),
				wall_placement_from_centered(center_xz, yaw, half_len, height, DEFAULT_THICK),
			),
			floor: FloorNode::rough_stone(
				Floor::rectangle(),
				Placement::new(center_xz, 0.0).with_scale(floor_scale),
			),
			constraints,
		}
	}
}

impl LodScene for WizardsTowerRoom {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		_level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(self.partition.scene_with_lod(lod_ref)),
			Box::new(self.floor.scene_with_lod(lod_ref)),
		];
		scene_children(children)
	}
}
