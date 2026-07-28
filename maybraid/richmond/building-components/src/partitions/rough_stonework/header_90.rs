//! Header-height 90° rough stonework arc.

use crate::assets::partitions::rough_stonework::HEADER_90;
use crate::partitions::geometry_components::WallComponent;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkHeader90;

impl From<WallComponent> for RoughStoneworkHeader90 {
	fn from(_: WallComponent) -> Self {
		Self
	}
}

crate::impl_glb_lod_scene!(RoughStoneworkHeader90, HEADER_90);
