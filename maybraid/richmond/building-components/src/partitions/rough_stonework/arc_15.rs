//! 15° angular rough stonework partition for curved door/window framing.

use crate::assets::partitions::rough_stonework::{ARC_15_HIGH, ARC_15_LOW, ARC_15_MID};
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;

/// Narrow arc sweep used to compose circular openings.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStonework15;

impl_partition_mesh_lod_scene!(
	RoughStonework15,
	PartitionMeshSet::new(ARC_15_HIGH, ARC_15_MID, ARC_15_LOW)
);
