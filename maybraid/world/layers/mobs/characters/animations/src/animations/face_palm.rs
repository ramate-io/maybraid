//! One-shot face palm: raise a relaxed arm across toward the brow, hold, recover.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

pub const RAISE_END: f32 = 0.20;
pub const CONTACT_END: f32 = 0.34;
pub const HOLD_END: f32 = 0.64;
const HUMERUS_INBOARD: f32 = 0.72;
const HUMERUS_UP: f32 = 0.98;
const HUMERUS_FORWARD: f32 = 0.08;
const ELBOW_CONTACT: f32 = 1.88;
const PALM_ROLL: f32 = FRAC_PI_2;
const NECK_TILT_BACK: f32 = 0.10;
const SPINE_LEAN_BACK: f32 = 0.05;
const SHOULDER_LIFT: f32 = 0.26;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FacePalm {
	pub side: Side,
}

impl Default for FacePalm {
	fn default() -> Self {
		Self { side: Side::Right }
	}
}

impl FacePalm {
	pub fn new(side: Side) -> Self {
		Self { side }
	}
	pub fn with_side(mut self, side: Side) -> Self {
		self.side = side;
		self
	}

	pub fn strike_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < RAISE_END {
			smoothstep(t / RAISE_END) * 0.55
		} else if t < CONTACT_END {
			0.55 + 0.45 * smoothstep((t - RAISE_END) / (CONTACT_END - RAISE_END))
		} else if t < HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - HOLD_END) / (1.0 - HOLD_END))
		}
	}

	pub fn elbow_flex(&self) -> f32 {
		ELBOW_CONTACT
	}

	pub fn humerus_along(&self) -> Vec3 {
		Vec3::new(-self.side.sign() * HUMERUS_INBOARD, HUMERUS_UP, HUMERUS_FORWARD).normalize()
	}

	pub fn palm_roll(&self) -> f32 {
		PALM_ROLL * self.side.sign()
	}
	pub fn shoulder_lift(&self) -> f32 {
		-SHOULDER_LIFT * self.side.sign()
	}
	pub fn neck_tilt_back(&self) -> f32 {
		-NECK_TILT_BACK
	}
	pub fn spine_lean_back(&self) -> f32 {
		-SPINE_LEAN_BACK
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn face_palm_clamps_and_reaches_contact_hold() -> anyhow::Result<()> {
		let palm = FacePalm::default();
		assert_eq!(palm.strike_amount(-1.0), 0.0);
		assert_eq!(palm.strike_amount(2.0), 0.0);
		assert!(palm.strike_amount(0.0) < 1e-6);
		assert!(palm.strike_amount(0.48) > 0.99);
		let right = FacePalm::new(Side::Right);
		let left = FacePalm::new(Side::Left);
		assert!(right.humerus_along().x > 0.0 && left.humerus_along().x < 0.0);
		assert!((right.humerus_along().x + left.humerus_along().x).abs() < 1e-6);
		assert!((right.palm_roll() + left.palm_roll()).abs() < 1e-6);
		Ok(())
	}
}
