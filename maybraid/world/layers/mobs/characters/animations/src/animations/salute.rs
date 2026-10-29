//! Salute: raise one relaxed arm beside the head, hold, and recover.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

pub const RAISE_END: f32 = 0.20;
pub const HOLD_END: f32 = 0.70;
const SALUTE_ELBOW: f32 = 1.45;
const HUMERUS_UP: f32 = 0.82;
const HUMERUS_INBOARD: f32 = 0.48;
const HUMERUS_FORWARD: f32 = 0.12;
const SALUTE_ROLL: f32 = FRAC_PI_2;
const NECK_TURN: f32 = 0.08;
const NECK_NOD: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Salute {
	pub side: Side,
}

impl Default for Salute {
	fn default() -> Self {
		Self { side: Side::Right }
	}
}

impl Salute {
	pub fn new(side: Side) -> Self {
		Self { side }
	}

	pub fn with_side(mut self, side: Side) -> Self {
		self.side = side;
		self
	}

	pub fn opposite_side(&self) -> Side {
		self.side.opposite()
	}

	pub fn salute_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < RAISE_END {
			smoothstep(t / RAISE_END)
		} else if t < HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - HOLD_END) / (1.0 - HOLD_END))
		}
	}

	/// Full inboard and upward aim, mirrored using the production side convention.
	pub fn humerus_along(&self) -> Vec3 {
		Vec3::new(-self.side.sign() * HUMERUS_INBOARD, HUMERUS_UP, HUMERUS_FORWARD).normalize()
	}

	pub fn salute_roll(&self) -> f32 {
		SALUTE_ROLL * self.side.sign()
	}

	pub fn salute_elbow(&self) -> f32 {
		SALUTE_ELBOW
	}

	pub fn neck_turn(&self) -> f32 {
		NECK_TURN * self.side.sign()
	}

	pub fn neck_nod(&self) -> f32 {
		NECK_NOD
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn salute_clamps_and_mirrors_its_full_aim() -> anyhow::Result<()> {
		let right = Salute::new(Side::Right);
		let left = Salute::new(Side::Left);
		assert_eq!(right.salute_amount(-1.0), 0.0);
		assert_eq!(right.salute_amount(2.0), 0.0);
		assert!(right.salute_amount(0.0) < 1e-6);
		assert!(right.salute_amount(0.45) > 0.99);
		let r = right.humerus_along();
		let l = left.humerus_along();
		assert!(r.x > 0.0 && l.x < 0.0, "both hands aim inboard {r:?} {l:?}");
		assert!(r.y > 0.7 && r.z > 0.0);
		assert!((r.x + l.x).abs() < 1e-6);
		assert!((right.salute_roll() + left.salute_roll()).abs() < 1e-6);
		Ok(())
	}
}
