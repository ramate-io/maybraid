//! "Think Again" meme gesture: raised upper arm, forearm ticks beside the head.
//!
//! Character space matches [`Jab`](super::Jab): +X right, +Y up, +Z fight-forward.
//! The humerus abducts laterally and lifts ~10° as the elbow folds. Cadence is
//! four beats: Think, rest, rest, Again.
//!
//! Forearm tilt in tests is derived from posed elbow→tip positions: `atan2(Δx, Δy)`.
//! Think ≈ +45° inboard; Again ≈ +15° inboard.

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

/// Four-beat clip: Think | rest | rest | Again.
const BEAT: f32 = 0.25;
/// Think occupies beat 1 and a short release into beat 2.
const THINK_END: f32 = BEAT + 0.05;
/// Again occupies beat 4, with a matching attack out of beat 3.
const AGAIN_START: f32 = 3.0 * BEAT - 0.05;
/// Raise / drop. Matched so the hand does not pop on either edge.
const ATTACK: f32 = 0.10;
const RELEASE: f32 = 0.10;

/// Elbow flex at Think. World tilt is ~45° inboard after the 10° humerus lift.
const THINK_ELBOW: f32 = std::f32::consts::FRAC_PI_2 + 35_f32.to_radians();
/// Elbow flex at Again. World tilt is ~+15° inboard after the same lift.
const AGAIN_ELBOW: f32 = std::f32::consts::FRAC_PI_2 + 5_f32.to_radians();
/// Humerus lift from the T-pose horizontal, in radians, at full gesture.
const HUMERUS_ELEVATION: f32 = 10_f32.to_radians();
/// Roll about the lateral humerus so the forearm hinge opens beside the head.
///
/// Not multiplied by [`Side::sign`]: bilateral mirroring is already carried by
/// [`Self::humerus_along`] and the rig's mirrored shoulder rests. See rig tests.
const THINK_AGAIN_ROLL: f32 = 0.0;

/// One-shot Think Again knobs. The rig resolver maps these onto anatomical frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThinkAgain {
	pub side: Side,
}

impl Default for ThinkAgain {
	fn default() -> Self {
		Self { side: Side::Right }
	}
}

impl ThinkAgain {
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

	/// Beat-1 pulse. Zero on the rest beats and on Again.
	pub fn think_amount(&self, progress: f32) -> f32 {
		beat_pulse(Progress(progress).clamp(), 0.0, THINK_END, ATTACK, RELEASE)
	}

	/// Beat-4 pulse. Zero on Think and the two rest beats.
	pub fn again_amount(&self, progress: f32) -> f32 {
		beat_pulse(Progress(progress).clamp(), AGAIN_START, 1.0, ATTACK, RELEASE)
	}

	/// Overall gesture envelope: either word, nothing on the rest beats.
	pub fn gesture_amount(&self, progress: f32) -> f32 {
		self.think_amount(progress).max(self.again_amount(progress))
	}

	/// Forearm flex in radians. Think and Again are separate pulses, not a sweep.
	pub fn elbow_flexion(&self, progress: f32) -> f32 {
		THINK_ELBOW * self.think_amount(progress) + AGAIN_ELBOW * self.again_amount(progress)
	}

	/// Character-space humerus length direction: lateral abduction plus a small lift.
	///
	/// [`Side::sign`] is `+1` on left and `−1` on right (yaw/roll mirroring). Lateral
	/// outboard matches rest geometry: right → `−X`, left → `+X`. Elevation tracks
	/// the elbow envelope so the upper arm leaves the T-pose as the forearm folds.
	pub fn humerus_along(&self, progress: f32) -> Vec3 {
		let amount = self.gesture_amount(progress);
		let elev = HUMERUS_ELEVATION * amount;
		Vec3::new(self.side.sign() * elev.cos(), elev.sin(), 0.0).normalize_or_zero()
	}

	pub fn humerus_roll(&self, _progress: f32) -> f32 {
		THINK_AGAIN_ROLL
	}
}

/// Smooth attack, hold, and release on `[start, end)`.
fn beat_pulse(t: f32, start: f32, end: f32, attack: f32, release: f32) -> f32 {
	if t <= start || t >= end {
		return 0.0;
	}
	let local = t - start;
	let dur = end - start;
	let release_at = (dur - release).max(attack);
	if local < attack {
		smoothstep(local / attack)
	} else if local < release_at {
		1.0
	} else {
		1.0 - smoothstep((local - release_at) / (dur - release_at).max(1e-4))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn think_hold() -> f32 {
		0.15
	}

	fn rest_hold() -> f32 {
		0.50
	}

	fn again_hold() -> f32 {
		0.85
	}

	#[test]
	fn think_again_starts_and_ends_at_rest() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!(clip.gesture_amount(0.0) < 1e-4);
		assert!(clip.gesture_amount(1.0) < 1e-4);
		assert!(clip.elbow_flexion(0.0) < 1e-4);
		assert!(clip.elbow_flexion(1.0) < 0.05);
		Ok(())
	}

	#[test]
	fn think_again_reaches_think_hold() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!(clip.think_amount(think_hold()) > 0.99);
		assert!(clip.again_amount(think_hold()) < 1e-4);
		assert!((clip.elbow_flexion(think_hold()) - THINK_ELBOW).abs() < 0.02);
		Ok(())
	}

	#[test]
	fn think_again_rest_beats_are_at_rest() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!(clip.gesture_amount(rest_hold()) < 1e-4);
		assert!(clip.elbow_flexion(0.375) < 1e-4);
		assert!(clip.elbow_flexion(0.625) < 1e-4);
		Ok(())
	}

	#[test]
	fn think_again_reaches_again_hold() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!(clip.again_amount(again_hold()) > 0.99);
		assert!(clip.think_amount(again_hold()) < 1e-4);
		assert!((clip.elbow_flexion(again_hold()) - AGAIN_ELBOW).abs() < 0.02);
		Ok(())
	}

	#[test]
	fn think_again_humerus_lifts_with_the_elbow() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let along = clip.humerus_along(think_hold());
		let elev = along.y.atan2(along.x.abs()).to_degrees();
		assert!(along.x.abs() > 0.95, "stays mostly lateral, got {along:?}");
		assert!(
			(elev - 10.0).abs() < 0.5,
			"humerus should lift ~10° with the elbow, got {elev:.1}° {along:?}"
		);
		Ok(())
	}

	#[test]
	fn think_again_lateral_sign_follows_side() -> anyhow::Result<()> {
		let right = ThinkAgain::default().with_side(Side::Right);
		let left = ThinkAgain::default().with_side(Side::Left);
		let r = right.humerus_along(think_hold());
		let l = left.humerus_along(think_hold());
		assert!(r.x < 0.0, "right arm lateral −X, got {r:?}");
		assert!(l.x > 0.0, "left arm lateral +X, got {l:?}");
		assert!((r.x + l.x).abs() < 0.05, "mirrored lateral aim {r:?} {l:?}");
		Ok(())
	}

	#[test]
	fn think_again_again_is_shallower_than_think() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		let delta = clip.elbow_flexion(think_hold()) - clip.elbow_flexion(again_hold());
		assert!(
			(delta - 30_f32.to_radians()).abs() < 0.05,
			"expected 30° Think→Again, got {delta}"
		);
		Ok(())
	}

	#[test]
	fn think_again_clamps_beyond_one_shot_end() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!((clip.gesture_amount(1.5) - clip.gesture_amount(1.0)).abs() < 1e-4);
		Ok(())
	}
}
