//! Bilateral shoulder shrug: lift both shoulders, bend elbows up, palms out, recover.
//!
//! A symmetric "I don't know" gesture on the T-pose rest (`for_clip_test`). The arms
//! are already abducted; the read is shoulder elevation plus elbow flex that raises
//! the forearm tips. Distinct from single-arm aim clips and overhead celebration waves.

use bevy::prelude::Vec3;

use crate::animations::smoothstep;
use crate::Progress;

const RAISE_END: f32 = 0.22;
const HOLD_END: f32 = 0.68;

/// Shoulder lift (parent flex) at full shrug.
const SHOULDER_LIFT: f32 = 0.55;
/// Elbow flex so forearms sit near horizontal, palms up.
const ELBOW_BEND: f32 = 1.35;
/// Light neck side tilt during the hold.
const NECK_SIDE_TILT: f32 = 0.06;

/// One-shot shrug knobs. The rig resolver maps these onto V0 channels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Shrug;

impl Shrug {
	/// Shrug envelope: raise, hold, recover. Clamped one-shot in `[0, 1]`.
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

	pub fn shoulder_lift(&self, progress: f32) -> f32 {
		SHOULDER_LIFT * self.shrug_amount(progress)
	}

	pub fn elbow_bend(&self, progress: f32) -> f32 {
		ELBOW_BEND * self.shrug_amount(progress)
	}

	pub fn neck_side_tilt(&self, progress: f32) -> f32 {
		NECK_SIDE_TILT * self.shrug_amount(progress)
	}

	/// Slight upward humerus aim so both elbows rise symmetrically off the T-pose.
	pub fn humerus_along(&self, progress: f32) -> Vec3 {
		let amount = self.shrug_amount(progress);
		Vec3::new(0.0, 0.82 * amount, 0.18 * amount).normalize()
	}

	/// Shared arm channels for both sides. Flexion signs mirror in the resolver.
	pub fn arm_channels(&self, progress: f32) -> (f32, f32) {
		(self.shoulder_lift(progress), self.elbow_bend(progress))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn peak() -> f32 {
		(RAISE_END + HOLD_END) * 0.5
	}

	#[test]
	fn shrug_starts_and_ends_at_rest() -> anyhow::Result<()> {
		let shrug = Shrug;
		assert!(shrug.shrug_amount(0.0) < 1e-4);
		assert!(shrug.shrug_amount(1.0) < 1e-4);
		assert!(shrug.shoulder_lift(0.0) < 1e-4);
		assert!(shrug.shoulder_lift(1.0) < 0.05);
		Ok(())
	}

	#[test]
	fn shrug_reaches_full_hold() -> anyhow::Result<()> {
		let shrug = Shrug;
		assert!(shrug.shrug_amount(peak()) > 0.99);
		assert!(shrug.shoulder_lift(peak()) > SHOULDER_LIFT * 0.95);
		assert!(shrug.elbow_bend(peak()) > ELBOW_BEND * 0.95);
		Ok(())
	}

	#[test]
	fn shrug_amount_is_monotonic_on_raise() -> anyhow::Result<()> {
		let shrug = Shrug;
		let early = shrug.shrug_amount(RAISE_END * 0.25);
		let late = shrug.shrug_amount(RAISE_END * 0.75);
		assert!(late > early);
		assert!(early > 0.0);
		Ok(())
	}

	#[test]
	fn shrug_clamps_beyond_one_shot_end() -> anyhow::Result<()> {
		let shrug = Shrug;
		assert!((shrug.shrug_amount(1.5) - shrug.shrug_amount(1.0)).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn shrug_channels_are_shared_for_both_sides() -> anyhow::Result<()> {
		let shrug = Shrug;
		let p = peak();
		let channels = shrug.arm_channels(p);
		assert!(channels.0 > 0.4, "shoulder lift peaks");
		assert!(channels.1 > 1.0, "elbow bends");
		Ok(())
	}
}
