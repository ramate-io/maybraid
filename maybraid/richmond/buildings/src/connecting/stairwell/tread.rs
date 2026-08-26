//! Last-tread contact for an upper landing (flight-agnostic).

use bevy_math::Vec2;

use crate::stair_flights::SpiralFlight;

/// Leading edge and travel of the last tread — enough to author a rim landing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreadEnd {
	pub leading_outer: Vec2,
	pub leading_inner: Vec2,
	pub travel: Vec2,
}

impl TreadEnd {
	pub fn from_spiral(flight: &SpiralFlight) -> Self {
		let (leading_outer, leading_inner) = flight.last_tread_leading_xz();
		Self { leading_outer, leading_inner, travel: flight.last_tread_travel_xz() }
	}

	/// Vector along the leading edge, outer → inner.
	pub fn leading(self) -> Vec2 {
		self.leading_inner - self.leading_outer
	}
}
