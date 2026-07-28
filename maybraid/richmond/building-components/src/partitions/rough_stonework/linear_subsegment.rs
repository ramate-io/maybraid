//! Linear rough stonework subsegment (normalized \(X \in [-1, 0.8]\)).

use crate::partitions::geometry_components::WallComponent;

/// Partial-length linear wall used beside openings.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkLinearSubsegment;

impl From<WallComponent> for RoughStoneworkLinearSubsegment {
	fn from(_: WallComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStoneworkLinearSubsegment);
