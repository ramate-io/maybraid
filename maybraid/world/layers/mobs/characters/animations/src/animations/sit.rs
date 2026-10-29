//! Seated biped idle: ground or seat plant, plus a quiet breath loop.
//!
//! Progress is unwrapped time. The sampled pose is always seated; settle-in
//! and stand-up are [`super::Transition::from_visible`] blends from the last
//! visible pose and armature offset. Angles are anatomical starting points —
//! posed joints on the bind are the contract.

use std::f32::consts::{FRAC_PI_2, TAU};

use character_rigs::humanoid::LegSegmentLengths;

use crate::animations::squat::vertical_drop;

/// Quiet chest cycle, in cycles per unit of unwrapped progress.
pub const SIT_BREATH_FREQ: f32 = 0.28;

/// Where the hips plant. Clip knob, not a rig bind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SitSurface {
	Ground,
	Seat,
}

/// Looping seated idle. [`Self::ground`] and [`Self::seat`] are the authored looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sit {
	pub surface: SitSurface,
	/// Peak femur forward swing (radians). Negative is the V0 hip-fold sign.
	pub femur_peak: f32,
	/// Peak shin flex relative to femur (radians).
	pub shin_peak: f32,
	/// Pelvis crease (radians).
	pub hip_peak: f32,
	/// Sagittal spine lean at rest breath (radians). Keep smaller than held squat.
	pub spine_peak: f32,
	/// Knee spread as femur abduction (radians).
	pub hip_spread: f32,
	/// Humerus flex that drops the arms toward the lap (radians).
	pub arm_hang: f32,
	/// Elbow bend on the lap (radians).
	pub elbow_hang: f32,
	/// Shoulder swing that brings the hands in front of the thighs (radians).
	pub shoulder_forward: f32,
	/// Spine pitch amplitude of the breath (radians).
	pub breath_amp: f32,
	/// Extra visual Y drop on top of the two-link fold.
	pub drop_bias: f32,
	/// Extra Y bob from the breath (meters). Keep tiny.
	pub breath_drop: f32,
	/// Skip [`crate::Effects::move`] so physics can own the origin snap.
	pub bones_only: bool,
}

impl Sit {
	/// Knees-up floor sit: hips closer to the ground than a chair.
	pub fn ground() -> Self {
		Self {
			surface: SitSurface::Ground,
			femur_peak: -1.62,
			shin_peak: 1.48,
			hip_peak: 0.72,
			spine_peak: 0.20,
			hip_spread: 0.16,
			arm_hang: 0.95,
			elbow_hang: 0.88,
			shoulder_forward: 0.14,
			breath_amp: 0.035,
			drop_bias: 0.10,
			breath_drop: 0.012,
			bones_only: false,
		}
	}

	/// Chair sit: ~90° hip and knee, more upright torso.
	pub fn seat() -> Self {
		Self {
			surface: SitSurface::Seat,
			femur_peak: -FRAC_PI_2,
			shin_peak: FRAC_PI_2,
			hip_peak: 0.42,
			spine_peak: 0.08,
			hip_spread: 0.05,
			arm_hang: 0.85,
			elbow_hang: 0.78,
			shoulder_forward: 0.10,
			breath_amp: 0.028,
			drop_bias: 0.0,
			breath_drop: 0.008,
			bones_only: false,
		}
	}

	pub fn breath(progress: f32) -> f32 {
		(TAU * progress * SIT_BREATH_FREQ).sin()
	}

	pub fn femur_swing(&self) -> f32 {
		self.femur_peak
	}

	pub fn shin_flex(&self) -> f32 {
		self.shin_peak
	}

	pub fn spine_pitch(&self, progress: f32) -> f32 {
		self.spine_peak + Self::breath(progress) * self.breath_amp
	}

	pub fn vertical_drop(&self, progress: f32, lengths: LegSegmentLengths) -> f32 {
		let fold = vertical_drop(self.femur_swing(), self.shin_flex(), lengths);
		fold + self.drop_bias + Self::breath(progress) * self.breath_drop
	}
}

impl Default for Sit {
	fn default() -> Self {
		Self::ground()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ground_folds_more_and_drops_more_than_seat() {
		let ground = Sit::ground();
		let seat = Sit::seat();
		let lengths = LegSegmentLengths::default();
		assert!(ground.femur_peak.abs() > seat.femur_peak.abs());
		assert!(ground.spine_peak > seat.spine_peak);
		assert!(ground.hip_spread > seat.hip_spread);
		assert!(ground.vertical_drop(0.0, lengths) > seat.vertical_drop(0.0, lengths) + 0.05);
	}

	#[test]
	fn seat_starts_near_a_right_angle() {
		let seat = Sit::seat();
		assert!((seat.femur_peak.abs() - FRAC_PI_2).abs() < 1e-5);
		assert!((seat.shin_peak - FRAC_PI_2).abs() < 1e-5);
	}

	#[test]
	fn breath_is_a_closed_loop() {
		let a = Sit::breath(0.0);
		let b = Sit::breath(1.0 / SIT_BREATH_FREQ);
		assert!((a - b).abs() < 1e-5);
		assert!(Sit::breath(0.25 / SIT_BREATH_FREQ).abs() > 0.9);
	}

	#[test]
	fn breath_does_not_rewrite_the_leg_fold() {
		let sit = Sit::ground();
		assert_eq!(sit.femur_swing(), sit.femur_peak);
		assert_eq!(sit.shin_flex(), sit.shin_peak);
	}
}
