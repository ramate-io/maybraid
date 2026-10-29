//! One-shot transition from fixed upright Idle(0) into the held prone pose.

use crate::animations::smoothstep;
use crate::Progress;

pub const DEFAULT_PRONE_DESCENT_SPEED: f32 = 2.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProneDescent;

impl ProneDescent {
	pub fn depth(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn clamped_depth_has_eased_one_shot_endpoints() -> anyhow::Result<()> {
		let descent = ProneDescent;
		assert_eq!(descent.depth(-1.0), 0.0);
		assert_eq!(descent.depth(0.0), 0.0);
		assert_eq!(descent.depth(1.0), 1.0);
		assert_eq!(descent.depth(2.0), 1.0);
		assert!((descent.depth(0.5) - 0.5).abs() < 1e-6);
		Ok(())
	}
}
