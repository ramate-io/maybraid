//! Header-height 15° rough stonework arc for curved door frames.

use crate::assets::partitions::rough_stonework::{HEADER_15_HIGH, HEADER_15_LOW, HEADER_15_MID};
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkHeader15;

impl_partition_mesh_lod_scene!(
	RoughStoneworkHeader15,
	PartitionMeshSet::new(HEADER_15_HIGH, HEADER_15_MID, HEADER_15_LOW)
);
