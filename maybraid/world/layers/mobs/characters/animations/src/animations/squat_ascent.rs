//! One-shot transition from held squat to fixed upright Idle(0).

use std::f32::consts::PI;

use crate::animations::smoothstep;
use crate::Progress;
use character_rigs::Side;

pub const DEFAULT_SQUAT_ASCENT_SPEED: f32 = 2.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SquatAscent;

impl SquatAscent {
	/// Remaining squat depth: one at held bottom, zero at upright.
	pub fn depth(&self, progress: f32) -> f32 {
		1.0 - smoothstep(Progress(progress).clamp())
	}

	/// Brief shoulder lead, clear at endpoints and when toggled at half depth.
	pub fn shoulder_lead(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		let bell = (PI * t).sin();
		let distance = (2.0 * (self.depth(t) - 0.5)).abs().min(1.0);
		0.12 * bell * bell * distance.powi(4)
	}

	pub fn shoulder_flex(&self, side: Side, progress: f32) -> f32 {
		self.shoulder_lead(progress) * side.sign()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn depth_and_sided_lead_reach_clean_endpoints() -> anyhow::Result<()> {
		let ascent = SquatAscent;
		assert!((ascent.depth(0.0) - 1.0).abs() < 1e-6);
		assert!(ascent.depth(1.0).abs() < 1e-6);
		assert!((ascent.depth(-1.0) - 1.0).abs() < 1e-6);
		assert!(ascent.depth(2.0).abs() < 1e-6);
		assert!(ascent.shoulder_lead(0.0).abs() < 1e-6);
		assert!(ascent.shoulder_lead(0.5).abs() < 1e-6);
		assert!(ascent.shoulder_lead(1.0).abs() < 1e-6);
		assert!(
			ascent.shoulder_flex(Side::Left, 0.15) * ascent.shoulder_flex(Side::Right, 0.15) < 0.0
		);
		Ok(())
	}
}
