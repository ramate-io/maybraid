//! Continuous stair geometry.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stair {
	Spiral(SpiralStair),
	Straight(StraightStair),
}

impl Stair {
	pub fn spiral() -> Self {
		Self::Spiral(SpiralStair)
	}

	pub fn straight() -> Self {
		Self::Straight(StraightStair)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpiralStair;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StraightStair;
