//! Continuous furniture / fixture forms (placeholder kit).

use bevy::prelude::Color;

/// Furniture or bathroom-fixture form filled by a wireframe box for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FurnitureGeometry {
	#[default]
	Bed,
	Wardrobe,
	Dresser,
	Nightstand,
	BedroomFurniture,
	Toilet,
	Chair,
	Chest,
	Counter,
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
			Self::Toilet => Color::srgba(0.85, 0.85, 0.90, 0.85),
			Self::Chair => Color::srgba(0.90, 0.55, 0.25, 0.85),
			Self::Chest => Color::srgba(0.55, 0.35, 0.70, 0.85),
			Self::Counter => Color::srgba(0.40, 0.75, 0.80, 0.85),
		}
	}

	/// Discriminant mixed into [`super::FurnitureNode::finish_seed`].
	pub const fn finish_salt(self) -> u64 {
		match self {
			Self::Bed => 0x6265_6400,
			Self::Wardrobe => 0x7761_7264,
			Self::Dresser => 0x6472_6573,
			Self::Nightstand => 0x6e69_6768,
			Self::BedroomFurniture => 0x6265_6472,
			Self::Toilet => 0x746f_696c,
			Self::Chair => 0x6368_6169,
			Self::Chest => 0x6368_6573,
			Self::Counter => 0x636f_756e,
		}
	}

	/// All kinds (for wireframe material registration).
	pub const ALL: [Self; 9] = [
		Self::Bed,
		Self::Wardrobe,
		Self::Dresser,
		Self::Nightstand,
		Self::BedroomFurniture,
		Self::Toilet,
		Self::Chair,
		Self::Chest,
		Self::Counter,
	];
}
