use crate::doors::geometry_components::DoorComponent;

/// Wood door leaf hung in a stone (or wood) frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WoodDoorLeaf;

impl From<DoorComponent> for WoodDoorLeaf {
	fn from(_: DoorComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(WoodDoorLeaf);
