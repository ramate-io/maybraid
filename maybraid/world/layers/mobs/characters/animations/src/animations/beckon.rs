//! One-shot beckon: reach forward and curl one arm in a short repeated pulse.

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

pub const EXTEND_END: f32 = 0.22;
pub const BECKON_END: f32 = 0.76;
const BECKON_CYCLES: f32 = 2.25;
const HUMERUS_FORWARD: f32 = 0.78;
const HUMERUS_LATERAL: f32 = 0.38;
const HUMERUS_UP: f32 = 0.22;
const EXTEND_ELBOW: f32 = 0.38;
const BECKON_ELBOW: f32 = 0.55;
const CURL_AMP: f32 = 0.24;
const CURL_EXTEND: f32 = 0.22;
const CURL_AIM_UP: f32 = 0.32;
const SHOULDER_LIFT: f32 = 0.28;
const TORSO_TURN: f32 = 0.10;
const NECK_TURN: f32 = 0.07;
const BECKON_ROLL: f32 = FRAC_PI_2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beckon {
	pub side: Side,
}

impl Default for Beckon {
	fn default() -> Self {
		Self { side: Side::Right }
	}
}

impl Beckon {
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

	pub fn gesture_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < EXTEND_END {
			smoothstep(t / EXTEND_END)
		} else if t < BECKON_END {
			1.0
		} else {
			1.0 - smoothstep((t - BECKON_END) / (1.0 - BECKON_END))
		}
	}

	pub fn curl_wave(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < EXTEND_END || t > BECKON_END {
			return 0.0;
		}
		let u = (t - EXTEND_END) / (BECKON_END - EXTEND_END);
		let edge = 0.16;
		let envelope = if u < edge {
			smoothstep(u / edge)
		} else if u > 1.0 - edge {
			smoothstep((1.0 - u) / edge)
		} else {
			1.0
		};
		(TAU * BECKON_CYCLES * u).sin() * envelope
	}

	pub fn humerus_along(&self, progress: f32) -> Vec3 {
		let curl = self.curl_wave(progress).abs();
		Vec3::new(
			self.side.sign() * HUMERUS_LATERAL,
			HUMERUS_UP + CURL_AIM_UP * curl,
			HUMERUS_FORWARD,
		)
		.normalize()
	}

	pub fn beckon_roll(&self) -> f32 {
		BECKON_ROLL * self.side.sign()
	}

	pub fn elbow_flex(&self, progress: f32) -> f32 {
		let amount = self.gesture_amount(progress);
		if amount <= 1e-4 {
			return 0.0;
		}
		let base = EXTEND_ELBOW + (BECKON_ELBOW - EXTEND_ELBOW) * amount;
		let curl = self.curl_wave(progress).abs();
		// The rig adapter applies `gesture_amount` as its outer blend weight.
		// Divide that weight out so the resolved elbow matches the source channel.
		(base * amount + CURL_AMP * curl - CURL_EXTEND * (1.0 - curl) * amount) / amount
	}

	pub fn shoulder_lift(&self, progress: f32) -> f32 {
		-SHOULDER_LIFT * self.curl_wave(progress).abs() * self.side.sign()
	}

	pub fn torso_turn(&self) -> f32 {
		TORSO_TURN * self.side.sign()
	}
	pub fn neck_turn(&self) -> f32 {
		NECK_TURN * self.side.sign()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn beckon_clamps_and_curl_is_bounded_to_hold() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		assert_eq!(beckon.gesture_amount(-1.0), 0.0);
		assert_eq!(beckon.gesture_amount(2.0), 0.0);
		assert!(beckon.gesture_amount(0.48) > 0.99);
		assert_eq!(beckon.curl_wave(0.0), 0.0);
		assert_eq!(beckon.curl_wave(1.0), 0.0);
		let early_curl = EXTEND_END + (BECKON_END - EXTEND_END) * 0.35;
		let late_curl = EXTEND_END + (BECKON_END - EXTEND_END) * 0.65;
		assert!(beckon.curl_wave(early_curl).abs() > 0.1);
		assert!(beckon.curl_wave(late_curl).abs() > 0.1);
		let right = beckon.humerus_along(0.48);
		let left = Beckon::new(Side::Left).humerus_along(0.48);
		assert!(right.z > 0.55 && right.x < 0.0);
		assert!((right.x + left.x).abs() < 1e-6);
		Ok(())
	}
}
