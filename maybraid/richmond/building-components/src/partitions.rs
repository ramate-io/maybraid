//! Wall / partition scene components (linear, angular, and header variants).
//!
//! IR: [`WallStyle`] + [`WallGeometry`] + [`Placement`] → [`WallNode`] (`LodScene`).

pub mod geometry;
pub mod lod;
pub mod node;
pub mod rough_stonework;
pub mod style;
pub(crate) mod tessellate;

pub use geometry::*;
pub use lod::{
	PartitionLodBand, PartitionMeshSet, PartitionMeshTier, PARTITION_HIGH_FACTOR,
	PARTITION_LOW_FACTOR, PARTITION_MEDIUM_FACTOR,
};
pub use node::WallNode;
pub use rough_stonework::*;
pub use style::WallStyle;
