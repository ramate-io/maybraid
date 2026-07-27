use crate::floors::geometry_components::FloorComponent;

/// Rectangular rough stone floor fill.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneFloorRectangle;

impl From<FloorComponent> for RoughStoneFloorRectangle {
	fn from(_: FloorComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStoneFloorRectangle);
