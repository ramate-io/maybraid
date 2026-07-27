use crate::roofs::geometry_components::RoofComponent;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WoodPerchDeck;

impl From<RoofComponent> for WoodPerchDeck {
	fn from(_: RoofComponent) -> Self {
		Self
	}
}

crate::impl_empty_lod_scene!(WoodPerchDeck);
