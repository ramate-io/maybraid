//! Bilateral shoulder shrug: lift both shoulders and turn both forearms forward.

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

pub const RAISE_END: f32 = 0.22;
pub const HOLD_END: f32 = 0.68;
const SHOULDER_LIFT: f32 = 0.32;
const ELBOW_BEND: f32 = 1.55;
const NECK_SIDE_TILT: f32 = 0.06;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Shrug;

impl Shrug {
	pub fn shrug_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < RAISE_END {
			smoothstep(t / RAISE_END)
		} else if t < HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - HOLD_END) / (1.0 - HOLD_END))
		}
	}

	pub fn shoulder_lift(&self, side: Side) -> f32 {
		SHOULDER_LIFT * side.sign()
	}

	pub fn elbow_bend(&self) -> f32 {
		ELBOW_BEND
	}

	pub fn neck_side_tilt(&self) -> f32 {
		NECK_SIDE_TILT
	}

	/// Lowered outboard humerus; the flexed forearm turns forward/up toward the chest.
	pub fn humerus_along(&self, side: Side) -> Vec3 {
		Vec3::new(side.sign() * 0.30, -0.72, 0.62).normalize()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn shrug_envelope_clamps_and_reaches_hold() -> anyhow::Result<()> {
		let shrug = Shrug;
		assert_eq!(shrug.shrug_amount(-1.0), shrug.shrug_amount(0.0));
		assert_eq!(shrug.shrug_amount(2.0), shrug.shrug_amount(1.0));
		assert!(shrug.shrug_amount(0.0) < 1e-6);
		assert!(shrug.shrug_amount(1.0) < 1e-6);
		assert!(shrug.shrug_amount(0.45) > 0.99);
		assert!(shrug.shoulder_lift(Side::Left) * shrug.shoulder_lift(Side::Right) < 0.0);
		let left = shrug.humerus_along(Side::Left);
		let right = shrug.humerus_along(Side::Right);
		assert!((left.x + right.x).abs() < 1e-6);
		assert!((left.y - right.y).abs() < 1e-6);
		assert!((left.z - right.z).abs() < 1e-6);
		Ok(())
	}
}
