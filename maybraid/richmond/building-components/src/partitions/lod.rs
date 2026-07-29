//! Partition LOD: probe re-exports and parent warm-host entry point.
//!
//! Banding policy lives on geometry modules ([`LinearLod`](crate::partitions::geometry::LinearLod),
//! [`JointLod`](crate::partitions::geometry::JointLod)). Shared host BSN is in [`host`](crate::partitions::host).

pub use crate::partitions::geometry::{
	LINEAR_HIGH_FACTOR, LINEAR_LOW_FACTOR, LINEAR_MEDIUM_FACTOR,
};
pub use crate::partitions::host::warm_content_host;
pub use crate::partitions::mesh_set::{PartitionMeshSet, PartitionMeshTier};
pub use crate::partitions::probe::{
	update_partition_host_levels, PartitionLodBand, PartitionLodProbe,
};
