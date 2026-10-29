//! One-shot transition from held prone to fixed upright Idle(0).

use std::f32::consts::PI;

use crate::animations::smoothstep;
use crate::Progress;

pub const DEFAULT_PRONE_ASCENT_SPEED: f32 = 2.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProneAscent;

impl ProneAscent {
	/// Remaining prone amount: one at held prone, zero at upright.
	pub fn depth(&self, progress: f32) -> f32 {
		1.0 - smoothstep(Progress(progress).clamp())
	}

	/// Brief arm push, gated away from the mid-depth reversal point and endpoints.
	pub fn arm_push(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		let depth = self.depth(t);
		let bell = (PI * t).sin();
		0.22 * bell * bell * (2.0 * (depth - 0.5)).abs().min(1.0).powi(4)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn depth_and_push_are_bounded_and_continuous_at_endpoints() -> anyhow::Result<()> {
		let ascent = ProneAscent;
		assert!((ascent.depth(-1.0) - 1.0).abs() < 1e-6);
		assert!((ascent.depth(0.0) - 1.0).abs() < 1e-6);
		assert!(ascent.depth(1.0).abs() < 1e-6);
		assert!(ascent.depth(2.0).abs() < 1e-6);
		assert!(ascent.arm_push(0.0).abs() < 1e-6);
		assert!(ascent.arm_push(1.0).abs() < 1e-6);
		assert!(ascent.arm_push(0.2).is_finite());
		assert!(ascent.arm_push(0.2) > 0.0);
		assert_eq!(ascent.arm_push(0.5), 0.0);
		Ok(())
	}
}
