//! Partition scene components (linear, angular, and header variants).
//!
//! IR: [`PartitionStyle`] + [`PartitionGeometry`] + [`Placement`] → [`PartitionNode`] (`LodScene`).
//! Primitive kit geometry — portal-sensitive walls live in `richmond_buildings::walling`.

pub mod geometry;
pub mod host;
pub mod lod;
pub mod mesh_set;
pub mod node;
pub mod probe;
pub mod rough_stonework;
pub mod style;

pub use geometry::*;
pub use lod::{
	update_partition_host_levels, PartitionLodBand, PartitionLodProbe, PartitionMeshSet,
	PartitionMeshTier, LINEAR_HIGH_FACTOR, LINEAR_LOW_FACTOR, LINEAR_MEDIUM_FACTOR,
};
pub use node::PartitionNode;
pub use rough_stonework::*;
pub use style::PartitionStyle;
