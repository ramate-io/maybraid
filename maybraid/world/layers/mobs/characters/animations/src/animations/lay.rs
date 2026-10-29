//! Recumbent biped idle: back or side, plus a quiet breath loop.
//!
//! Progress is unwrapped time. The sampled pose is always lying; lie-down and
//! get-up are [`super::Transition::from_visible`] blends from the last visible
//! pose and armature offset. The body goes horizontal through
//! [`character_rigs::authoring::ArmatureOffset`] rotation, not a spine pretzel.
//! That offset is visual only.

use std::f32::consts::TAU;

use character_rigs::Side;

/// Quiet chest cycle, in cycles per unit of unwrapped progress.
pub const LAY_BREATH_FREQ: f32 = 0.24;

/// Recumbent orientation. Clip knob, not a rig bind.
///
/// Side-lying carries the down-shoulder. The back has none, so
/// [`Lay::back`] does not invent a dummy [`Side`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayOrientation {
	Back,
	Side(Side),
}

impl LayOrientation {
	/// Shoulder toward the ground. `None` when supine.
	pub const fn down_side(self) -> Option<Side> {
		match self {
			Self::Back => None,
			Self::Side(side) => Some(side),
		}
	}
}

/// Looping lying idle. [`Self::back`] and [`Self::side`] are the authored looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lay {
	pub orientation: LayOrientation,
	/// Light hip fold so the legs are not a standing T (radians).
	pub hip_flex: f32,
	/// Residual knee bend (radians).
	pub knee_flex: f32,
	/// Knee stack / spread as femur abduction (radians).
	pub hip_spread: f32,
	/// Arm hang off the T-pose (radians).
	pub arm_hang: f32,
	/// Elbow rest bend (radians).
	pub elbow_hang: f32,
	/// Small spine pitch; on the back this is the chest-up breath base.
	pub spine_peak: f32,
	/// Neck nod so the head is not a T-pose stare (radians).
	pub neck_peak: f32,
	/// Neck turn; side-lying uses this to keep the face readable.
	pub neck_turn: f32,
	/// Spine pitch amplitude of the breath (radians).
	pub breath_amp: f32,
	/// Extra visual Y drop so the torso is not hovering (meters).
	pub ground_drop: f32,
	/// Extra Y bob from the breath (meters).
	pub breath_drop: f32,
	/// Skip [`crate::Effects::move`] so physics can own the origin snap.
	pub bones_only: bool,
}

impl Lay {
	/// Supine: face up, legs slightly bent, arms resting.
	pub fn back() -> Self {
		Self {
			orientation: LayOrientation::Back,
			hip_flex: -0.28,
			knee_flex: 0.42,
			hip_spread: 0.06,
			arm_hang: 0.70,
			elbow_hang: 0.38,
			spine_peak: 0.06,
			neck_peak: 0.10,
			neck_turn: 0.0,
			breath_amp: 0.04,
			ground_drop: 0.12,
			breath_drop: 0.010,
			bones_only: false,
		}
	}

	/// Recumbent on `side` (that shoulder toward the ground).
	pub fn side(side: Side) -> Self {
		Self {
			orientation: LayOrientation::Side(side),
			hip_flex: -0.45,
			knee_flex: 0.72,
			hip_spread: 0.18,
			arm_hang: 0.55,
			elbow_hang: 0.55,
			spine_peak: 0.04,
			neck_peak: 0.08,
			neck_turn: -side.sign() * 0.22,
			breath_amp: 0.03,
			ground_drop: 0.10,
			breath_drop: 0.008,
			bones_only: false,
		}
	}

	pub fn breath(progress: f32) -> f32 {
		(TAU * progress * LAY_BREATH_FREQ).sin()
	}

	pub fn spine_pitch(&self, progress: f32) -> f32 {
		self.spine_peak + Self::breath(progress) * self.breath_amp
	}

	pub fn visual_drop(&self, progress: f32) -> f32 {
		self.ground_drop + Self::breath(progress) * self.breath_drop
	}

	/// Shoulder toward the ground. `None` when supine.
	pub const fn down_side(&self) -> Option<Side> {
		self.orientation.down_side()
	}
}

impl Default for Lay {
	fn default() -> Self {
		Self::back()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn side_tucks_more_than_back() {
		let back = Lay::back();
		let side = Lay::side(Side::Right);
		assert!(side.knee_flex > back.knee_flex);
		assert!(side.hip_flex.abs() > back.hip_flex.abs());
		assert!(side.hip_spread > back.hip_spread);
	}

	#[test]
	fn breath_is_a_closed_loop() {
		let a = Lay::breath(0.0);
		let b = Lay::breath(1.0 / LAY_BREATH_FREQ);
		assert!((a - b).abs() < 1e-5);
		assert!(Lay::breath(0.25 / LAY_BREATH_FREQ).abs() > 0.9);
	}

	#[test]
	fn side_neck_turn_follows_the_down_shoulder() {
		let left = Lay::side(Side::Left);
		let right = Lay::side(Side::Right);
		assert!(left.neck_turn < 0.0);
		assert!(right.neck_turn > 0.0);
		assert!((left.neck_turn + right.neck_turn).abs() < 1e-5);
	}

	#[test]
	fn orientation_carries_side_only_when_lying_on_a_side() {
		assert_eq!(Lay::back().orientation, LayOrientation::Back);
		assert_eq!(Lay::back().down_side(), None);
		assert_eq!(Lay::side(Side::Left).orientation, LayOrientation::Side(Side::Left));
		assert_eq!(Lay::side(Side::Right).orientation, LayOrientation::Side(Side::Right));
		assert_eq!(Lay::side(Side::Left).down_side(), Some(Side::Left));
		assert_eq!(Lay::side(Side::Right).down_side(), Some(Side::Right));
	}
}
