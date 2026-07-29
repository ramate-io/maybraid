//! Partition scene components (linear, angular, and header variants).
//!
//! IR: [`PartitionStyle`] + [`PartitionGeometry`] + [`Placement`] → [`PartitionNode`] (`LodScene`).
//! Primitive kit geometry — portal-sensitive walls live in `richmond_buildings::walling`.

pub mod geometry;
pub mod lod;
pub mod node;
pub mod rough_stonework;
pub mod style;
pub(crate) mod tessellate;

pub use geometry::*;
pub use lod::{
	update_partition_host_levels, PartitionLodBand, PartitionLodProbe, PartitionMeshSet,
	PartitionMeshTier, PARTITION_HIGH_FACTOR, PARTITION_LOW_FACTOR, PARTITION_MEDIUM_FACTOR,
};
pub use node::PartitionNode;
pub use rough_stonework::*;
pub use style::PartitionStyle;
