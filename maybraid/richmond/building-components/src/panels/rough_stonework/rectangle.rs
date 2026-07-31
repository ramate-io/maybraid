//! Rough-stonework unit rectangle panel kit (LOD triad).

use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;

use crate::assets::panels::rough_stonework::{RECTANGLE_HIGH, RECTANGLE_LOW, RECTANGLE_MID};
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;
use crate::roofs::lod::leaf_scene_ref_lod;

/// Unit rectangle \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStonePanelRectangle;

impl_partition_mesh_lod_scene!(
	RoughStonePanelRectangle,
	PartitionMeshSet::new(RECTANGLE_HIGH, RECTANGLE_MID, RECTANGLE_LOW)
);

impl RoughStonePanelRectangle {
	/// LOD host for panel leaves (same triad as [`Self`] as `LodScene`).
	pub fn scene_with_lod(lod_ref: &LodRef) -> impl Scene + 'static {
		leaf_scene_ref_lod(
			RECTANGLE_HIGH.scene_ref(),
			RECTANGLE_MID.scene_ref(),
			RECTANGLE_LOW.scene_ref(),
			lod_ref,
		)
	}
}
