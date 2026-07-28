use crate::stairs::geometry_components::StairComponent;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WoodStraightStair;

impl From<StairComponent> for WoodStraightStair {
	fn from(_: StairComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(WoodStraightStair);
