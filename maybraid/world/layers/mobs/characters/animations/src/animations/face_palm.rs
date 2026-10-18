//! One-shot face palm: raise one arm and slap the palm to the brow, brief hold, recover.
//!
//! Body-space aim matches [`Jab`](super::Jab): +X right, +Y up, +Z fight-forward.
//! Distinct from waves, salutes, and think-again clips: the read is a crossed-arm
//! forehead contact with the head tipping back slightly.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

const RAISE_END: f32 = 0.20;
const CONTACT_END: f32 = 0.34;
const HOLD_END: f32 = 0.64;

const HUMERUS_INBOARD: f32 = 0.72;
const HUMERUS_UP: f32 = 0.98;
const HUMERUS_FORWARD: f32 = 0.08;

const ELBOW_CONTACT: f32 = 1.88;
const PALM_ROLL: f32 = FRAC_PI_2;

const NECK_TILT_BACK: f32 = 0.10;
const SPINE_LEAN_BACK: f32 = 0.05;
const SHOULDER_LIFT: f32 = 0.26;

/// One-shot face-palm knobs. The rig resolver maps these onto anatomical frames.
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

	/// Reach envelope: raise, slap contact, hold, recover. Clamped one-shot in `[0, 1]`.
	pub fn strike_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < RAISE_END {
			smoothstep(t / RAISE_END) * 0.55
		} else if t < CONTACT_END {
			let u = (t - RAISE_END) / (CONTACT_END - RAISE_END);
			0.55 + 0.45 * smoothstep(u)
		} else if t < HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - HOLD_END) / (1.0 - HOLD_END))
		}
	}

	pub fn elbow_flex(&self, progress: f32) -> f32 {
		ELBOW_CONTACT * self.strike_amount(progress)
	}

	/// Character-space humerus length direction for the palm arm (toward the brow).
	pub fn humerus_along(&self, progress: f32) -> Vec3 {
		let amount = self.strike_amount(progress);
		if amount < 1e-4 {
			return Vec3::Y;
		}
		let inboard = -self.side.sign() * HUMERUS_INBOARD * amount;
		Vec3::new(inboard, HUMERUS_UP * amount, HUMERUS_FORWARD * amount).normalize()
	}

	pub fn palm_roll(&self) -> f32 {
		PALM_ROLL
	}

	pub fn shoulder_lift(&self, progress: f32) -> f32 {
		-SHOULDER_LIFT * self.strike_amount(progress) * self.side.sign()
	}

	pub fn neck_tilt_back(&self, progress: f32) -> f32 {
		-NECK_TILT_BACK * self.strike_amount(progress)
	}

	pub fn spine_lean_back(&self, progress: f32) -> f32 {
		-SPINE_LEAN_BACK * self.strike_amount(progress)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn peak() -> f32 {
		(CONTACT_END + HOLD_END) * 0.5
	}

	#[test]
	fn face_palm_starts_and_ends_at_rest() -> anyhow::Result<()> {
		let palm = FacePalm::default();
		assert!(palm.strike_amount(0.0) < 1e-4);
		assert!(palm.strike_amount(1.0) < 1e-4);
		assert!(palm.elbow_flex(0.0) < 1e-4);
		assert!(palm.elbow_flex(1.0) < 0.08);
		Ok(())
	}

	#[test]
	fn face_palm_reaches_full_contact_hold() -> anyhow::Result<()> {
		let palm = FacePalm::default();
		assert!(palm.strike_amount(peak()) > 0.99);
		assert!(palm.elbow_flex(peak()) > ELBOW_CONTACT * 0.95);
		Ok(())
	}

	#[test]
	fn face_palm_humerus_aims_up_and_inboard_at_peak() -> anyhow::Result<()> {
		let palm = FacePalm::default().with_side(Side::Right);
		let along = palm.humerus_along(peak());
		assert!(along.y > 0.75, "brow reach is mostly +Y, got {along:?}");
		assert!(along.x > 0.45, "right palm crosses inboard (+X aim), got {along:?}");
		Ok(())
	}

	#[test]
	fn face_palm_clamps_beyond_one_shot_end() -> anyhow::Result<()> {
		let palm = FacePalm::default();
		assert!((palm.strike_amount(1.5) - palm.strike_amount(1.0)).abs() < 1e-5);
		Ok(())
	}
}
