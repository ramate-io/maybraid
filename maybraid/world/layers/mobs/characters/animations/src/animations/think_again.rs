//! "Think Again" meme gesture: lateral upper arm, forearm sweeps beside the head.
//!
//! Character space matches [`Jab`](super::Jab): +X right, +Y up, +Z fight-forward.
//! The humerus stays abducted horizontally; Think and Again differ mainly in elbow flex.
//!
//! Angle interpretation (see PR): forearm tilt from vertical in the arm plane (X–Y when
//! the humerus points lateral). Think ≈ −45° inboard; Again ≈ +15° outboard (~60° sweep).

use std::f32::consts::PI;

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

/// Raise into Think completes; first hold beat ends.
const RAISE_END: f32 = 0.18;
/// Second hold beat ends (Think pose held steady through [`AGAIN_END`]).
const HOLD2_END: f32 = 0.58;
/// Again key pose reached.
const AGAIN_END: f32 = 0.72;
/// Brief Again hold ends; recover begins.
const AGAIN_HOLD_END: f32 = 0.82;

/// Elbow flex at Think (−45° from vertical on the tuned roll axis).
const THINK_ELBOW: f32 = 2.35;
/// Elbow flex at Again (+15° from vertical).
const AGAIN_ELBOW: f32 = 1.30;
/// Roll so palm faces inward once the humerus is lateral.
const THINK_AGAIN_ROLL: f32 = PI;

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

	/// Overall gesture envelope: up through Again hold, down on recover.
	pub fn gesture_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < RAISE_END {
			smoothstep(t / RAISE_END)
		} else if t < AGAIN_HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - AGAIN_HOLD_END) / (1.0 - AGAIN_HOLD_END))
		}
	}

	/// Forearm flex in radians. Sweeps Think → Again between [`AGAIN_END`] and [`AGAIN_HOLD_END`].
	pub fn elbow_flexion(&self, progress: f32) -> f32 {
		self.pose_elbow(progress) * self.gesture_amount(progress)
	}

	fn pose_elbow(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < AGAIN_END {
			THINK_ELBOW
		} else if t < AGAIN_HOLD_END {
			let u = smoothstep((t - AGAIN_END) / (AGAIN_HOLD_END - AGAIN_END));
			THINK_ELBOW + (AGAIN_ELBOW - THINK_ELBOW) * u
		} else {
			AGAIN_ELBOW
		}
	}

	/// Character-space humerus length direction: lateral abduction (half T-pose).
	///
	/// [`Side::sign`] is `+1` on left and `−1` on right (yaw/roll mirroring). Lateral
	/// outboard is the opposite: right → `+X`, left → `−X`.
	pub fn humerus_along(&self, progress: f32) -> Vec3 {
		let amount = self.gesture_amount(progress);
		Vec3::new(-self.side.sign() * amount, 0.0, 0.0).normalize_or_zero()
	}

	pub fn humerus_roll(&self, _progress: f32) -> f32 {
		THINK_AGAIN_ROLL
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn think_hold() -> f32 {
		(RAISE_END + HOLD2_END) * 0.5
	}

	fn again_hold() -> f32 {
		0.815
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
		assert!(clip.gesture_amount(think_hold()) > 0.99);
		assert!((clip.elbow_flexion(think_hold()) - THINK_ELBOW).abs() < 0.02);
		Ok(())
	}

	#[test]
	fn think_again_reaches_again_hold() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!((clip.elbow_flexion(again_hold()) - AGAIN_ELBOW).abs() < 0.02);
		Ok(())
	}

	#[test]
	fn think_again_humerus_stays_lateral_at_peak() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let along = clip.humerus_along(think_hold());
		assert!(along.x.abs() > 0.95, "lateral abduction, got {along:?}");
		assert!(along.y.abs() < 0.08, "humerus stays horizontal, got {along:?}");
		Ok(())
	}

	#[test]
	fn think_again_lateral_sign_follows_side() -> anyhow::Result<()> {
		let right = ThinkAgain::default().with_side(Side::Right);
		let left = ThinkAgain::default().with_side(Side::Left);
		let r = right.humerus_along(think_hold());
		let l = left.humerus_along(think_hold());
		assert!(r.x > 0.0, "right arm lateral +X, got {r:?}");
		assert!(l.x < 0.0, "left arm lateral −X, got {l:?}");
		assert!((r.x + l.x).abs() < 0.05, "mirrored lateral aim {r:?} {l:?}");
		Ok(())
	}

	#[test]
	fn think_again_elbow_sweeps_about_sixty_degrees() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		let delta = clip.elbow_flexion(think_hold()) - clip.elbow_flexion(again_hold());
		assert!(delta > 0.9 && delta < 1.2, "expected ~1.05 rad sweep, got {delta}");
		Ok(())
	}

	#[test]
	fn think_again_clamps_beyond_one_shot_end() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		assert!((clip.gesture_amount(1.5) - clip.gesture_amount(1.0)).abs() < 1e-4);
		Ok(())
	}
}
