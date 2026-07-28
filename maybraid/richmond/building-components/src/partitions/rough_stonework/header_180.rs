//! Header-height 180° rough stonework arc.

use crate::partitions::geometry_components::WallComponent;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkHeader180;

impl From<WallComponent> for RoughStoneworkHeader180 {
	fn from(_: WallComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStoneworkHeader180);
