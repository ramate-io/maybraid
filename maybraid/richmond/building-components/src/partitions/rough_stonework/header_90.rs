//! Header-height 90° rough stonework arc.

use crate::assets::partitions::rough_stonework::{HEADER_90_HIGH, HEADER_90_LOW, HEADER_90_MID};
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkHeader90;

impl_partition_mesh_lod_scene!(
	RoughStoneworkHeader90,
	PartitionMeshSet::new(HEADER_90_HIGH, HEADER_90_MID, HEADER_90_LOW)
);
