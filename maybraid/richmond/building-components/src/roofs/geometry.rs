//! Continuous roof / cap geometry.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoofGeometry {
	Spire(SpireRoof),
	Perch(PerchRoof),
	Deck(PerchDeck),
}

impl RoofGeometry {
	pub fn spire() -> Self {
		Self::Spire(SpireRoof)
	}

	pub fn perch() -> Self {
		Self::Perch(PerchRoof)
	}

	pub fn deck() -> Self {
		Self::Deck(PerchDeck)
	}
}

/// Alias kept for migration; prefer [`RoofGeometry`].
pub type Roof = RoofGeometry;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpireRoof;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PerchRoof;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PerchDeck;
