use crate::assets::partitions::rough_stonework::LINEAR;
use crate::floors::geometry_components::FloorComponent;

/// Rectangular rough stone floor fill.
///
/// No dedicated floor-rectangle GLB yet — reuses the rough-stonework linear
/// partition mesh, remapped onto the floor plane in [`crate::floors::scene`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneFloorRectangle;

impl From<FloorComponent> for RoughStoneFloorRectangle {
	fn from(_: FloorComponent) -> Self {
		Self
	}
}

crate::impl_glb_lod_scene!(RoughStoneFloorRectangle, LINEAR);
