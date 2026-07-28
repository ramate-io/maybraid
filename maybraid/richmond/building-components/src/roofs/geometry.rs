//! Continuous roof / cap geometry.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Roof {
	Spire(SpireRoof),
	Perch(PerchRoof),
	Deck(PerchDeck),
}

impl Roof {
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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpireRoof;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PerchRoof;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PerchDeck;
