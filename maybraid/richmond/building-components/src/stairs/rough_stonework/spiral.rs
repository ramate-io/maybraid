use crate::stairs::geometry_components::StairComponent;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneSpiralStair;

impl From<StairComponent> for RoughStoneSpiralStair {
	fn from(_: StairComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStoneSpiralStair);
