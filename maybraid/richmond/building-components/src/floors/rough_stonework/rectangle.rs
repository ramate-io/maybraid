use crate::assets::floors::rough_stonework::RECTANGLE;
use crate::floors::geometry_components::FloorComponent;

/// Rectangular rough stone floor fill (`rough_stonework_001.glb`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneFloorRectangle;

impl From<FloorComponent> for RoughStoneFloorRectangle {
	fn from(_: FloorComponent) -> Self {
		Self
	}
}

crate::impl_glb_lod_scene!(RoughStoneFloorRectangle, RECTANGLE);
