//! Panel material style.

use crate::panels::geometry::PanelKitCaps;

/// Material look for shared panel geometry ([`crate::panels::PanelNode`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PanelStyle {
	#[default]
	RoughStonework,
	ShepherdsThatch,
}

impl PanelStyle {
	/// Kit capabilities used by [`crate::panels::PanelGeometry::flatten`].
	pub fn kit_caps(self) -> PanelKitCaps {
		PanelKitCaps::from(self)
	}
}

impl From<PanelStyle> for PanelKitCaps {
	fn from(style: PanelStyle) -> Self {
		match style {
			PanelStyle::RoughStonework => Self::WITH_RECTANGLE,
			PanelStyle::ShepherdsThatch => Self::TRIANGLES_ONLY,
		}
	}
}
