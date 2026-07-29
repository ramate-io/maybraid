//! Shepherd's-thatch unit right-triangle roof kit (LOD triad).

use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;
use scene_ref::MirrorAxis;

use crate::assets::roofs::shepherds_thatch::{
	RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_LOW, RIGHT_TRIANGLE_MID,
};
use crate::partitions::lod::{leaf_scene_ref_lod, PartitionMeshSet};
use crate::partitions::node::impl_partition_mesh_lod_scene;

/// Unit right triangle \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ShepherdsThatchRightTriangle;

impl_partition_mesh_lod_scene!(
	ShepherdsThatchRightTriangle,
	PartitionMeshSet::new(RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_MID, RIGHT_TRIANGLE_LOW)
);

impl ShepherdsThatchRightTriangle {
	/// LOD host with optional [`scene_ref::SceneRef`] axis mirroring on every tier.
	pub fn scene_with_lod_mirrored(
		lod_ref: &LodRef,
		mirror: Option<MirrorAxis>,
	) -> impl Scene + 'static {
		leaf_scene_ref_lod(
			RIGHT_TRIANGLE_HIGH.scene_ref().with_mirror(mirror),
			RIGHT_TRIANGLE_MID.scene_ref().with_mirror(mirror),
			RIGHT_TRIANGLE_LOW.scene_ref().with_mirror(mirror),
			lod_ref,
		)
	}
}
