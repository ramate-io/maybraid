//! Full-height linear rough stonework partition (normalized \(X \in [-1, 1]\)).

use crate::assets::partitions::rough_stonework::LINEAR;
use crate::partitions::geometry_components::WallComponent;

/// Linear wall segment for radial subdividers and straight partitions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneworkLinear;

impl From<WallComponent> for RoughStoneworkLinear {
	fn from(_: WallComponent) -> Self {
		Self
	}
}

crate::impl_glb_lod_scene!(RoughStoneworkLinear, LINEAR);
