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
	/// Furniture playground slot boxes (cyan = counter, gold = chair, …).
	///
	/// | Kind | Color | sRGBA |
	/// |---|---|---|
	/// | Bed | blue | `0.20, 0.55, 1.00, 1.0` |
	/// | Wardrobe | orange | `1.00, 0.55, 0.12, 1.0` |
	/// | Dresser | coral | `1.00, 0.38, 0.28, 1.0` |
	/// | Nightstand | green | `0.20, 1.00, 0.38, 1.0` |
	/// | BedroomFurniture | lime | `0.55, 1.00, 0.20, 1.0` |
	/// | Toilet | white | `0.95, 0.95, 1.00, 1.0` |
	/// | Chair | gold | `1.00, 0.78, 0.08, 1.0` |
	/// | Chest | magenta | `0.88, 0.28, 1.00, 1.0` |
	/// | Counter | cyan | `0.08, 0.95, 1.00, 1.0` |
	pub fn wireframe_color(self) -> Color {
		match self {
			Self::Bed => Color::srgba(0.20, 0.55, 1.00, 1.0),
			Self::Wardrobe => Color::srgba(1.00, 0.55, 0.12, 1.0),
			Self::Dresser => Color::srgba(1.00, 0.38, 0.28, 1.0),
			Self::Nightstand => Color::srgba(0.20, 1.00, 0.38, 1.0),
			Self::BedroomFurniture => Color::srgba(0.55, 1.00, 0.20, 1.0),
			Self::Toilet => Color::srgba(0.95, 0.95, 1.00, 1.0),
			Self::Chair => Color::srgba(1.00, 0.78, 0.08, 1.0),
			Self::Chest => Color::srgba(0.88, 0.28, 1.00, 1.0),
			Self::Counter => Color::srgba(0.08, 0.95, 1.00, 1.0),
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
