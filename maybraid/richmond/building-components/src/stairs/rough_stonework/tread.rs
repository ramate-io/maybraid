//! Single stair tread (`rough_stonework_tread_001.glb`).

use crate::assets::stairs::rough_stonework::TREAD;
use crate::stairs::geometry_components::StairComponent;

/// Unit tread cube (kit \(X = Y = Z \in [-1, 1]\), left face −Z).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneTread;

impl From<StairComponent> for RoughStoneTread {
	fn from(_: StairComponent) -> Self {
		Self
	}
}

crate::impl_glb_lod_scene!(RoughStoneTread, TREAD);
