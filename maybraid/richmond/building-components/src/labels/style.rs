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
			Self::Red => Color::srgba(1.00, 0.18, 0.18, 1.0),
			Self::Orange => Color::srgba(1.00, 0.62, 0.08, 1.0),
			Self::Yellow => Color::srgba(1.00, 0.92, 0.12, 1.0),
			Self::Green => Color::srgba(0.18, 1.00, 0.32, 1.0),
			Self::Cyan => Color::srgba(0.08, 0.95, 1.00, 1.0),
			Self::Blue => Color::srgba(0.22, 0.48, 1.00, 1.0),
			Self::Magenta => Color::srgba(1.00, 0.22, 1.00, 1.0),
			Self::Gray => Color::srgba(0.82, 0.82, 0.88, 1.0),
		}
	}

	/// Pick a style from a unit sample in \[0, 1\].
	pub fn from_unit(t: f32) -> Self {
		let i = ((t.clamp(0.0, 0.999) * Self::ALL.len() as f32) as usize).min(Self::ALL.len() - 1);
		Self::ALL[i]
	}
}
