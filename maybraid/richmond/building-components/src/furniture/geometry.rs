//! Continuous furniture / fixture forms (placeholder kit).

use bevy::prelude::Color;

/// Furniture or bathroom-fixture form filled by a wireframe box for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FurnitureGeometry {
	Bed,
	Wardrobe,
	Dresser,
	Nightstand,
	BedroomFurniture,
	Vanity,
	Toilet,
}

impl FurnitureGeometry {
	/// Debug wireframe color for this kind.
	pub fn wireframe_color(self) -> Color {
		match self {
			Self::Bed => Color::srgba(0.35, 0.55, 0.95, 0.85),
			Self::Wardrobe => Color::srgba(0.75, 0.45, 0.25, 0.85),
			Self::Dresser => Color::srgba(0.65, 0.40, 0.35, 0.85),
			Self::Nightstand => Color::srgba(0.45, 0.85, 0.50, 0.85),
			Self::BedroomFurniture => Color::srgba(0.55, 0.70, 0.45, 0.85),
			Self::Vanity => Color::srgba(0.85, 0.55, 0.85, 0.85),
			Self::Toilet => Color::srgba(0.85, 0.85, 0.90, 0.85),
		}
	}

	/// All kinds (for wireframe material registration).
	pub const ALL: [Self; 7] = [
		Self::Bed,
		Self::Wardrobe,
		Self::Dresser,
		Self::Nightstand,
		Self::BedroomFurniture,
		Self::Vanity,
		Self::Toilet,
	];
}
