use crate::floors::geometry_components::FloorComponent;

/// Rectangular wood floor fill.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WoodFloorRectangle;

impl From<FloorComponent> for WoodFloorRectangle {
	fn from(_: FloorComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(WoodFloorRectangle);
