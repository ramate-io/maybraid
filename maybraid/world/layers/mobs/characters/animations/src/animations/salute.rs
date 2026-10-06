//! Military salute: raise the hand to the brow, hold, recover.
//!
//! Body-space aim matches [`Jab`](super::Jab): +X right, +Y up, +Z fight-forward.
//! The saluting humerus sweeps inboard and up; the elbow flexes to bring the forearm
//! vertical at the temple. The opposite arm and legs stay at rest.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

const RAISE_END: f32 = 0.20;
const HOLD_END: f32 = 0.70;

/// Elbow flex at the brow (radians).
const SALUTE_ELBOW: f32 = 1.45;
/// Humerus aim weights at full salute.
const HUMERUS_UP: f32 = 0.82;
const HUMERUS_INBOARD: f32 = 0.48;
const HUMERUS_FORWARD: f32 = 0.12;
/// Roll so the forearm plane faces inward once aimed.
const SALUTE_ROLL: f32 = FRAC_PI_2;
/// Neck turn toward the saluting side at full hold.
const NECK_TURN: f32 = 0.08;
/// Neck nod so the brow meets the hand.
const NECK_NOD: f32 = 0.05;

/// One-shot salute knobs. The rig resolver maps these onto anatomical frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Salute {
	pub side: Side,
}

impl Default for Salute {
	fn default() -> Self {
		Self { side: Side::Right }
	}
}

impl Salute {
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

	/// Salute envelope: raise, hold, recover. Clamped one-shot in `[0, 1]`.
	pub fn salute_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < RAISE_END {
			smoothstep(t / RAISE_END)
		} else if t < HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - HOLD_END) / (1.0 - HOLD_END))
		}
	}

	/// Character-space humerus length direction for the saluting arm.
	pub fn humerus_along(&self, progress: f32) -> Vec3 {
		let amount = self.salute_amount(progress);
		let inboard = -self.side.sign() * HUMERUS_INBOARD;
		Vec3::new(inboard * amount, HUMERUS_UP * amount, HUMERUS_FORWARD * amount).normalize()
	}

	pub fn salute_roll(&self, _progress: f32) -> f32 {
		SALUTE_ROLL
	}

	pub fn salute_elbow(&self, progress: f32) -> f32 {
		SALUTE_ELBOW * self.salute_amount(progress)
	}

	pub fn neck_turn(&self, progress: f32) -> f32 {
		NECK_TURN * self.salute_amount(progress) * self.side.sign()
	}

	pub fn neck_nod(&self, progress: f32) -> f32 {
		NECK_NOD * self.salute_amount(progress)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn peak() -> f32 {
		(RAISE_END + HOLD_END) * 0.5
	}

	#[test]
	fn salute_starts_and_ends_at_rest() -> anyhow::Result<()> {
		let salute = Salute::default();
		assert!(salute.salute_amount(0.0) < 1e-4);
		assert!(salute.salute_amount(1.0) < 1e-4);
		assert!(salute.salute_elbow(0.0) < 1e-4);
		assert!(salute.salute_elbow(1.0) < 0.05);
		Ok(())
	}

	#[test]
	fn salute_reaches_full_hold() -> anyhow::Result<()> {
		let salute = Salute::default();
		assert!(salute.salute_amount(peak()) > 0.99);
		assert!(salute.salute_elbow(peak()) > SALUTE_ELBOW * 0.95);
		Ok(())
	}

	#[test]
	fn salute_humerus_aims_up_at_peak() -> anyhow::Result<()> {
		let salute = Salute::default().with_side(Side::Right);
		let along = salute.humerus_along(peak());
		assert!(along.y > 0.7, "salute aims up, got {along:?}");
		assert!(along.z > 0.0, "salute carries slight forward, got {along:?}");
		Ok(())
	}

	#[test]
	fn salute_inboard_sign_follows_side() -> anyhow::Result<()> {
		let right = Salute::default().with_side(Side::Right);
		let left = Salute::default().with_side(Side::Left);
		let r = right.humerus_along(peak());
		let l = left.humerus_along(peak());
		assert!(r.x > 0.0, "right salute moves inboard (+X), got {r:?}");
		assert!(l.x < 0.0, "left salute moves inboard (−X), got {l:?}");
		assert!((r.x + l.x).abs() < 0.05, "mirrored lateral aim {r:?} {l:?}");
		Ok(())
	}

	#[test]
	fn salute_amount_is_monotonic_on_raise() -> anyhow::Result<()> {
		let salute = Salute::default();
		let early = salute.salute_amount(RAISE_END * 0.25);
		let late = salute.salute_amount(RAISE_END * 0.75);
		assert!(late > early);
		Ok(())
	}

	#[test]
	fn salute_clamps_beyond_one_shot_end() -> anyhow::Result<()> {
		let salute = Salute::default();
		assert!((salute.salute_amount(1.5) - salute.salute_amount(1.0)).abs() < 1e-4);
		Ok(())
	}
}
