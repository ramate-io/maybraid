//! One-shot upright-to-held-prone descent.
//!
//! Connects upright locomotion ([`Still`](super::Idle), walk, run, held squat) to the held
//! [`Prone`](super::Prone) stance. Progress `0` is upright; `1` matches
//! [`Prone::default`] at full depth. Sampling is stateless: progress alone sets depth.

use crate::animations::{smoothstep, Prone};
use crate::Progress;

/// Default descent rate: full transition in ~0.35 s at speed `1.0`.
pub const DEFAULT_PRONE_DESCENT_SPEED: f32 = 2.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProneDescent;

impl ProneDescent {
	pub fn held() -> Self {
		Self
	}

	/// Eased prone depth: 0 at upright, 1 at held flat.
	pub fn depth(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	fn held_prone(&self) -> Prone {
		Prone::default()
	}

	pub fn spine_pitch(&self, progress: f32) -> f32 {
		self.held_prone().spine_pitch(progress)
	}

	pub fn femur_swing(&self, progress: f32) -> f32 {
		self.held_prone().femur_swing(progress)
	}

	pub fn shin_flex(&self, progress: f32) -> f32 {
		self.held_prone().shin_flex(progress)
	}

	pub fn neck_swing(&self, progress: f32) -> f32 {
		self.held_prone().neck_swing(progress)
	}

	pub fn arm_hold(&self, progress: f32) -> f32 {
		self.held_prone().arm_hold(progress)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn depth_endpoints_match_held_prone() -> anyhow::Result<()> {
		let descent = ProneDescent::default();
		assert!(descent.depth(0.0).abs() < 1e-5);
		assert!((descent.depth(1.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn depth_eases_in_the_middle() -> anyhow::Result<()> {
		let descent = ProneDescent::default();
		let mid = descent.depth(0.5);
		assert!(mid > 0.35 && mid < 0.65, "smoothstep midpoint, got {mid}");
		Ok(())
	}

	#[test]
	fn depth_clamps_past_one() -> anyhow::Result<()> {
		let descent = ProneDescent::default();
		assert!((descent.depth(1.5) - 1.0).abs() < 1e-5);
		Ok(())
	}
}
