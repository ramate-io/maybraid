//! One-shot held-squat-to-stand ascent.
//!
//! Connects the held [`Squat`](super::Squat) stance back to upright locomotion
//! ([`Still`](super::Idle), walk, run). Progress `0` matches [`Squat::held`];
//! `1` is upright. Sampling is stateless: progress alone sets depth.

use std::f32::consts::PI;

use crate::animations::{smoothstep, Squat};
use crate::Progress;

/// Default ascent rate: full transition in ~0.35 s at speed `1.0`.
pub const DEFAULT_ASCENT_SPEED: f32 = 2.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SquatAscent;

impl SquatAscent {
	pub fn held() -> Self {
		Self
	}

	/// Remaining squat depth: 1 at held bottom, 0 at upright.
	pub fn depth(&self, progress: f32) -> f32 {
		1.0 - smoothstep(Progress(progress).clamp())
	}

	fn held_squat(&self) -> Squat {
		Squat::held()
	}

	pub fn femur_swing(&self, progress: f32) -> f32 {
		let depth = self.depth(progress);
		self.held_squat().femur_peak * depth
	}

	pub fn shin_flex(&self, progress: f32) -> f32 {
		let depth = self.depth(progress);
		self.held_squat().shin_peak * depth
	}

	pub fn root_swing(&self, progress: f32) -> f32 {
		let depth = self.depth(progress);
		self.held_squat().root_peak * depth
	}

	pub fn hip_fold(&self, progress: f32) -> f32 {
		let depth = self.depth(progress);
		self.held_squat().hip_peak * depth
	}

	/// Brief bilateral shoulder lead while the legs extend (0 at endpoints).
	///
	/// Fades out near held depth so ascent↔descent toggles at matched depth stay continuous.
	pub fn shoulder_lead(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		let bell = (t * PI).sin();
		let raw = 0.12 * bell * bell;
		let depth = self.depth(progress);
		let away_from_mid_depth = (2.0 * (depth - 0.5)).abs().min(1.0);
		let guard = away_from_mid_depth.powi(4);
		raw * guard
	}

	/// Character-space shoulder lift; opposite signs so both shoulders rise (see Fall).
	pub fn shoulder_flex(&self, side: character_rigs::Side, progress: f32) -> f32 {
		self.shoulder_lead(progress) * side.sign()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn depth_endpoints_match_held_and_upright() -> anyhow::Result<()> {
		let ascent = SquatAscent::default();
		assert!((ascent.depth(0.0) - 1.0).abs() < 1e-5);
		assert!(ascent.depth(1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn depth_eases_in_the_middle() -> anyhow::Result<()> {
		let ascent = SquatAscent::default();
		let mid = ascent.depth(0.5);
		assert!(mid > 0.35 && mid < 0.65, "smoothstep midpoint, got {mid}");
		Ok(())
	}

	#[test]
	fn shoulder_flex_uses_opposite_signs_per_side() -> anyhow::Result<()> {
		use character_rigs::Side;

		let ascent = SquatAscent::default();
		assert!(
			ascent.shoulder_flex(Side::Left, 0.5).signum()
				!= ascent.shoulder_flex(Side::Right, 0.5).signum()
		);
		Ok(())
	}

	#[test]
	fn shoulder_lead_is_zero_at_endpoints() -> anyhow::Result<()> {
		let ascent = SquatAscent::default();
		assert!(ascent.shoulder_lead(0.0).abs() < 1e-5);
		assert!(ascent.shoulder_lead(1.0).abs() < 1e-5);
		assert!(ascent.shoulder_lead(0.5).abs() < 1e-5, "mid depth toggles clear shoulder lead");
		assert!(ascent.shoulder_lead(0.15) > 0.01);
		Ok(())
	}
}
