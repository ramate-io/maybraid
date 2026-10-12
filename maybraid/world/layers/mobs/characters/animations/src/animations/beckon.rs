//! One-shot beckon: extend one arm forward, curl the elbow in a short "come here" pulse, recover.
//!
//! Body-space aim matches [`Jab`](super::Jab): +X right, +Y up, +Z fight-forward.
//! Distinct from overhead waves, salutes, and spot scans: the read is a forward reach with
//! elbow flex oscillation during the hold.

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

const EXTEND_END: f32 = 0.22;
const BECKON_END: f32 = 0.76;
const BECKON_CYCLES: f32 = 2.25;

const HUMERUS_FORWARD: f32 = 0.78;
const HUMERUS_LATERAL: f32 = 0.38;
const HUMERUS_UP: f32 = 0.22;

const EXTEND_ELBOW: f32 = 0.42;
const BECKON_ELBOW: f32 = 0.62;
const CURL_AMP: f32 = 0.20;

const TORSO_TURN: f32 = 0.10;
const NECK_TURN: f32 = 0.07;
const BECKON_ROLL: f32 = FRAC_PI_2;

/// One-shot beckon knobs. The rig resolver maps these onto anatomical frames.
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

	/// Overall envelope: reach, beckon hold, recover. Clamped one-shot in `[0, 1]`.
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

	/// Elbow curl oscillation during the beckon window. Zero outside it.
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

	/// Character-space humerus length direction for the beckoning arm.
	pub fn humerus_along(&self, progress: f32) -> Vec3 {
		let amount = self.gesture_amount(progress);
		if amount < 1e-4 {
			return Vec3::Y;
		}
		let lateral = -self.side.sign() * HUMERUS_LATERAL * amount;
		Vec3::new(lateral, HUMERUS_UP * amount, HUMERUS_FORWARD * amount).normalize()
	}

	pub fn beckon_roll(&self) -> f32 {
		BECKON_ROLL
	}

	pub fn elbow_flex(&self, progress: f32) -> f32 {
		let amount = self.gesture_amount(progress);
		let base = EXTEND_ELBOW + (BECKON_ELBOW - EXTEND_ELBOW) * amount;
		base * amount + CURL_AMP * self.curl_wave(progress).abs()
	}

	pub fn torso_turn(&self, progress: f32) -> f32 {
		TORSO_TURN * self.gesture_amount(progress) * self.side.sign()
	}

	pub fn neck_turn(&self, progress: f32) -> f32 {
		NECK_TURN * self.gesture_amount(progress) * self.side.sign()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn peak() -> f32 {
		(EXTEND_END + BECKON_END) * 0.5
	}

	#[test]
	fn beckon_starts_and_ends_at_rest() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		assert!(beckon.gesture_amount(0.0) < 1e-4);
		assert!(beckon.gesture_amount(1.0) < 1e-4);
		assert!(beckon.elbow_flex(0.0) < 1e-4);
		assert!(beckon.elbow_flex(1.0) < 0.08);
		Ok(())
	}

	#[test]
	fn beckon_reaches_full_hold() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		assert!(beckon.gesture_amount(peak()) > 0.99);
		assert!(beckon.elbow_flex(peak()) > BECKON_ELBOW * 0.9);
		Ok(())
	}

	#[test]
	fn beckon_curl_oscillates_during_hold() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		let mid = EXTEND_END + (BECKON_END - EXTEND_END) * 0.35;
		let later = EXTEND_END + (BECKON_END - EXTEND_END) * 0.65;
		assert!(beckon.curl_wave(mid).abs() > 0.05);
		assert!(beckon.curl_wave(later).abs() > 0.05);
		assert!(beckon.curl_wave(0.0).abs() < 1e-4);
		assert!(beckon.curl_wave(1.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn beckon_humerus_points_forward_at_peak() -> anyhow::Result<()> {
		let beckon = Beckon::default().with_side(Side::Right);
		let along = beckon.humerus_along(peak());
		assert!(along.z > 0.55, "reach is fight-forward, got {along:?}");
		assert!(along.x > 0.0, "right beckon stays on +X, got {along:?}");
		assert!(along.z > 0.5, "reach is fight-forward, got {along:?}");
		Ok(())
	}

	#[test]
	fn beckon_clamps_beyond_one_shot_end() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		assert!((beckon.gesture_amount(1.5) - beckon.gesture_amount(1.0)).abs() < 1e-5);
		Ok(())
	}
}
