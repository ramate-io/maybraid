use crate::roofs::geometry_components::RoofComponent;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneSpireRoof;

impl From<RoofComponent> for RoughStoneSpireRoof {
	fn from(_: RoofComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStoneSpireRoof);
