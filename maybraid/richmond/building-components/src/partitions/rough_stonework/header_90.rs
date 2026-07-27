//! Header-height 90° rough stonework arc.

use crate::partitions::geometry_components::WallComponent;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkHeader90;

impl From<WallComponent> for RoughStoneworkHeader90 {
	fn from(_: WallComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStoneworkHeader90);
