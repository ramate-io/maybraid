//! Header-height linear rough stonework subsegment.

use crate::partitions::geometry_components::WallComponent;

/// Short vertical header segment for door/window frames on straight walls.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkLinearHeaderSubsegment;

impl From<WallComponent> for RoughStoneworkLinearHeaderSubsegment {
	fn from(_: WallComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStoneworkLinearHeaderSubsegment);
