use crate::floors::geometry_components::FloorComponent;

/// Occasional wood arc floor fill.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WoodFloorArcFill;

impl From<FloorComponent> for WoodFloorArcFill {
	fn from(_: FloorComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(WoodFloorArcFill);
