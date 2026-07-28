use crate::stairs::geometry_components::StairComponent;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneStraightStair;

impl From<StairComponent> for RoughStoneStraightStair {
	fn from(_: StairComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStoneStraightStair);
