//! 180° angular rough stonework partition for circular outer walls.

use crate::partitions::geometry_components::WallComponent;

/// Half-ring wall sweep through \(-Z\) from \(X = -1\) to \(X = 1\).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStonework180;

impl From<WallComponent> for RoughStonework180 {
	fn from(_: WallComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStonework180);
