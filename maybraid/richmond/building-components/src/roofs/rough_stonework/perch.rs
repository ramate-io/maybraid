use crate::roofs::geometry_components::RoofComponent;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStonePerchRoof;

impl From<RoofComponent> for RoughStonePerchRoof {
	fn from(_: RoofComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStonePerchRoof);
