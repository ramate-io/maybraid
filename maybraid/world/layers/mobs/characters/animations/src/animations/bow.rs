//! Respectful forward bow: dip the torso, soften the knees, arms at the thighs.
//!
//! Bilateral silhouette distinct from salutes, waves, and shrugs. Character space is
//! +X right, +Y up, +Z fight-forward. Progress is a one-shot in `[0, 1]`.

use bevy::prelude::Vec3;

use crate::animations::smoothstep;
use crate::Progress;

const DIP_END: f32 = 0.24;
const HOLD_END: f32 = 0.62;

/// Spine stack pitch at full bow (radians).
const SPINE_PITCH: f32 = 0.50;
/// Extra root lean so the fold reads in the playground camera.
const ROOT_LEAN: f32 = 0.14;
/// Modest knee bend so the bow is not a rigid hinge at the hips.
const FEMUR_SWING: f32 = 0.22;
const SHIN_FLEX: f32 = 0.30;
const HIP_FOLD: f32 = 0.10;
/// Head follows the torso without burying the face in the chest.
const NECK_NOD: f32 = 0.20;
/// Forearms slide toward the thighs from the T-pose rest.
const ELBOW_FLEX: f32 = 0.38;
const HUMERUS_DOWN: f32 = 0.72;
const HUMERUS_FORWARD: f32 = 0.28;

/// One-shot bow knobs. The rig resolver maps these onto V0 channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bow;

impl Bow {
	/// Bow envelope: dip, brief hold, recover.
	pub fn bow_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < DIP_END {
			smoothstep(t / DIP_END)
		} else if t < HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - HOLD_END) / (1.0 - HOLD_END))
		}
	}

	pub fn spine_pitch(&self, progress: f32) -> f32 {
		SPINE_PITCH * self.bow_amount(progress)
	}

	pub fn root_lean(&self, progress: f32) -> f32 {
		ROOT_LEAN * self.bow_amount(progress)
	}

	pub fn femur_swing(&self, progress: f32) -> f32 {
		FEMUR_SWING * self.bow_amount(progress)
	}

	pub fn shin_flex(&self, progress: f32) -> f32 {
		SHIN_FLEX * self.bow_amount(progress)
	}

	pub fn hip_fold(&self, progress: f32) -> f32 {
		HIP_FOLD * self.bow_amount(progress)
	}

	pub fn neck_nod(&self, progress: f32) -> f32 {
		NECK_NOD * self.bow_amount(progress)
	}

	pub fn elbow_flex(&self, progress: f32) -> f32 {
		ELBOW_FLEX * self.bow_amount(progress)
	}

	/// Character-space humerus direction: down and slightly forward at the thighs.
	pub fn humerus_along(&self, progress: f32) -> Vec3 {
		let amount = self.bow_amount(progress);
		Vec3::new(0.0, -HUMERUS_DOWN * amount, HUMERUS_FORWARD * amount).normalize_or_zero()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn peak() -> f32 {
		(DIP_END + HOLD_END) * 0.5
	}

	#[test]
	fn bow_starts_and_ends_at_rest() -> anyhow::Result<()> {
		let bow = Bow;
		assert!(bow.bow_amount(0.0) < 1e-4);
		assert!(bow.bow_amount(1.0) < 1e-4);
		assert!(bow.spine_pitch(0.0) < 1e-4);
		assert!(bow.spine_pitch(1.0) < 0.05);
		Ok(())
	}

	#[test]
	fn bow_reaches_full_hold() -> anyhow::Result<()> {
		let bow = Bow;
		assert!(bow.bow_amount(peak()) > 0.99);
		assert!(bow.spine_pitch(peak()) > SPINE_PITCH * 0.95);
		assert!(bow.shin_flex(peak()) > SHIN_FLEX * 0.95);
		Ok(())
	}

	#[test]
	fn bow_humerus_aims_down_at_peak() -> anyhow::Result<()> {
		let bow = Bow;
		let along = bow.humerus_along(peak());
		assert!(along.y < -0.5, "arms follow torso down, got {along:?}");
		assert!(along.z > 0.0, "slight forward carry, got {along:?}");
		Ok(())
	}

	#[test]
	fn bow_amount_is_monotonic_on_dip() -> anyhow::Result<()> {
		let bow = Bow;
		let early = bow.bow_amount(DIP_END * 0.25);
		let late = bow.bow_amount(DIP_END * 0.75);
		assert!(late > early);
		assert!(early > 0.0);
		Ok(())
	}

	#[test]
	fn bow_clamps_beyond_one_shot_end() -> anyhow::Result<()> {
		let bow = Bow;
		assert!((bow.bow_amount(1.5) - bow.bow_amount(1.0)).abs() < 1e-5);
		Ok(())
	}
}
