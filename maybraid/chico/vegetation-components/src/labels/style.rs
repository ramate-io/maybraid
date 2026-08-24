//! Label look: named debug colors.

use bevy::prelude::Color;

/// Named color styles for [`super::LabelNode`] placeholders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LabelStyle {
	#[default]
	Red,
	Orange,
	Yellow,
	Green,
	Cyan,
	Blue,
	Magenta,
	Gray,
}

impl LabelStyle {
	pub const ALL: [Self; 8] = [
		Self::Red,
		Self::Orange,
		Self::Yellow,
		Self::Green,
		Self::Cyan,
		Self::Blue,
		Self::Magenta,
		Self::Gray,
	];

	/// Wireframe stroke color for this style.
	pub fn color(self) -> Color {
		match self {
			Self::Red => Color::srgba(0.95, 0.3, 0.3, 0.9),
			Self::Orange => Color::srgba(0.98, 0.58, 0.22, 0.9),
			Self::Yellow => Color::srgba(0.95, 0.88, 0.28, 0.9),
			Self::Green => Color::srgba(0.35, 0.88, 0.42, 0.9),
			Self::Cyan => Color::srgba(0.25, 0.88, 0.92, 0.9),
			Self::Blue => Color::srgba(0.32, 0.48, 0.98, 0.9),
			Self::Magenta => Color::srgba(0.9, 0.38, 0.9, 0.9),
			Self::Gray => Color::srgba(0.7, 0.7, 0.75, 0.9),
		}
	}
}
