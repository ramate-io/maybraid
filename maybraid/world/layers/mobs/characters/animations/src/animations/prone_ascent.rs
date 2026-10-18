//! One-shot held-prone-to-upright ascent.
//!
//! Connects the held [`Prone`](super::Prone) stance back to upright locomotion
//! ([`Still`](super::Idle), walk, run). Progress `0` matches full prone depth;
//! `1` is upright. Sampling is stateless: progress alone sets depth.

use std::f32::consts::PI;

use crate::animations::{smoothstep, Prone};
use crate::Progress;

/// Default ascent rate: full transition in ~0.35 s at speed `1.0`.
pub const DEFAULT_PRONE_ASCENT_SPEED: f32 = 2.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProneAscent;

impl ProneAscent {
	pub fn held() -> Self {
		Self
	}

	fn held_prone(self) -> Prone {
		Prone::default()
	}

	/// Remaining prone depth: 1 at flat, 0 at upright.
	pub fn depth(&self, progress: f32) -> f32 {
		1.0 - smoothstep(Progress(progress).clamp())
	}

	pub fn spine_pitch(&self, progress: f32) -> f32 {
		self.held_prone().spine_pitch(self.depth(progress))
	}

	pub fn femur_swing(&self, progress: f32) -> f32 {
		self.depth(progress) * self.held_prone().femur_peak
	}

	pub fn shin_flex(&self, progress: f32) -> f32 {
		self.depth(progress) * self.held_prone().shin_peak
	}

	pub fn neck_swing(&self, progress: f32) -> f32 {
		self.depth(progress) * self.held_prone().neck_peak
	}

	pub fn arm_hold(&self, progress: f32) -> f32 {
		self.depth(progress) * self.held_prone().arm_flex
	}

	/// Brief push through the arms while the torso lifts (0 at endpoints).
	pub fn arm_push(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		let bell = (t * PI).sin();
		let raw = 0.22 * bell * bell;
		let depth = self.depth(progress);
		let guard = (2.0 * (depth - 0.5)).abs().min(1.0).powi(4);
		raw * guard
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn depth_endpoints_match_prone_and_upright() -> anyhow::Result<()> {
		let ascent = ProneAscent::default();
		assert!((ascent.depth(0.0) - 1.0).abs() < 1e-5);
		assert!(ascent.depth(1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn depth_eases_in_the_middle() -> anyhow::Result<()> {
		let ascent = ProneAscent::default();
		let mid = ascent.depth(0.5);
		assert!(mid > 0.35 && mid < 0.65, "smoothstep midpoint, got {mid}");
		Ok(())
	}

	#[test]
	fn arm_push_is_zero_at_endpoints() -> anyhow::Result<()> {
		let ascent = ProneAscent::default();
		assert!(ascent.arm_push(0.0).abs() < 1e-5);
		assert!(ascent.arm_push(1.0).abs() < 1e-5);
		assert!(ascent.arm_push(0.25) > 0.01);
		Ok(())
	}
}
