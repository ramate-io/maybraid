//! 90° angular rough stonework partition for circular outer walls.

use crate::partitions::geometry_components::WallComponent;

/// Quarter-ring wall sweep through \(-Z\) from \(X = -1\) to \(X = 0\).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStonework90;

impl From<WallComponent> for RoughStonework90 {
	fn from(_: WallComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStonework90);
