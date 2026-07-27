use crate::floors::geometry_components::FloorComponent;

/// Arc-segment floor fill in rough stone.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneFloorArcFill;

impl From<FloorComponent> for RoughStoneFloorArcFill {
	fn from(_: FloorComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStoneFloorArcFill);
