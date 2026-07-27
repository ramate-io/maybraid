//! 15° angular rough stonework partition for curved door/window framing.

use crate::partitions::geometry_components::WallComponent;

/// Narrow arc sweep used to compose circular openings.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStonework15;

impl From<WallComponent> for RoughStonework15 {
	fn from(_: WallComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStonework15);
