use crate::floors::geometry_components::FloorComponent;

/// Occasional wood structural floor bracing.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WoodFloorStructFill;

impl From<FloorComponent> for WoodFloorStructFill {
	fn from(_: FloorComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(WoodFloorStructFill);
