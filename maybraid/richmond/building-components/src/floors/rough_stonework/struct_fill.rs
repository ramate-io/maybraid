use crate::floors::geometry_components::FloorComponent;

/// Structural / radial floor bracing in rough stone.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneFloorStructFill;

impl From<FloorComponent> for RoughStoneFloorStructFill {
	fn from(_: FloorComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(RoughStoneFloorStructFill);
