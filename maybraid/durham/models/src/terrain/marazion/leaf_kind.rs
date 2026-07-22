//! Pocket-water leaf typing for debug overlays and HUD.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Color;

/// Resolved Marazion stamp on a guillotine leaf (after empty fallbacks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarazionLeafKind {
	/// Occupancy miss or every stamp recipe skipped.
	Empty,
	Stream,
	Bog,
	Lake,
}

impl MarazionLeafKind {
	/// Wire color for playground / gizmo overlays.
	pub fn debug_color(self) -> Color {
		match self {
			Self::Empty => Color::srgba(0.55, 0.55, 0.6, 0.35),
			Self::Stream => Color::srgb(0.15, 0.85, 0.95),
			Self::Bog => Color::srgb(0.55, 0.75, 0.2),
			Self::Lake => Color::srgb(0.2, 0.45, 1.0),
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Empty => "empty",
			Self::Stream => "stream",
			Self::Bog => "bog",
			Self::Lake => "lake",
		}
	}
}

/// Which Marazion occupancy band produced a leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarazionBandPass {
	Low,
	High,
}

impl MarazionBandPass {
	pub fn label(self) -> &'static str {
		match self {
			Self::Low => "low",
			Self::High => "high",
		}
	}
}

/// Leaf AABB + stamp kind retained on [`crate::terrain::Terrain`] for debug.
#[derive(Debug, Clone, Copy)]
pub struct MarazionLeafBounds {
	pub cell: Aabb3d,
	pub kind: MarazionLeafKind,
	pub band: MarazionBandPass,
}
