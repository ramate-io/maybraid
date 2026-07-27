//! Header-height 15° rough stonework arc for curved door frames.

use crate::assets::partitions::rough_stonework::HEADER_15;
use crate::partitions::geometry_components::WallComponent;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkHeader15;

impl From<WallComponent> for RoughStoneworkHeader15 {
	fn from(_: WallComponent) -> Self {
		Self
	}
}

crate::impl_glb_lod_scene!(RoughStoneworkHeader15, HEADER_15);
