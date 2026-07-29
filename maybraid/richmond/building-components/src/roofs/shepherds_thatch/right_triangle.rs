//! Shepherd's-thatch unit right-triangle roof kit (LOD triad).

use crate::assets::roofs::shepherds_thatch::{
	RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_LOW, RIGHT_TRIANGLE_MID,
};
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;

/// Unit right triangle \(X = Z = [0, 1]\), \(Y = [-0.2, 0.2]\).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ShepherdsThatchRightTriangle;

impl_partition_mesh_lod_scene!(
	ShepherdsThatchRightTriangle,
	PartitionMeshSet::new(RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_MID, RIGHT_TRIANGLE_LOW)
);
