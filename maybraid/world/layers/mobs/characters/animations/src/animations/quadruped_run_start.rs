//! One-shot quadruped still-to-trot launch.
//!
//! Connects [`QuadrupedIdle`](super::QuadrupedIdle) at neutral rest to the first diagonal
//! contact pose of [`QuadrupedRun`](super::QuadrupedRun) at cycle phase `0`. Progress `0` is
//! standing; `1` matches quadruped run@0. Sampling is stateless: progress alone sets weight.

use super::{smoothstep, QuadrupedIdle, QuadrupedRun};
use crate::Progress;

/// Default launch rate: full transition in ~0.28 s at speed `1.0`.
pub const DEFAULT_QUADRUPED_RUN_START_SPEED: f32 = 3.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadrupedRunStart {
	pub idle: QuadrupedIdle,
	pub run: QuadrupedRun,
}

impl Default for QuadrupedRunStart {
	fn default() -> Self {
		Self { idle: QuadrupedIdle::default(), run: QuadrupedRun::default() }
	}
}

impl QuadrupedRunStart {
	/// Eased blend weight: 0 at idle neutral, 1 at run phase zero.
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn weight_endpoints() -> anyhow::Result<()> {
		let start = QuadrupedRunStart::default();
		assert!(start.weight(0.0).abs() < 1e-5);
		assert!((start.weight(1.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn weight_eases_in_the_middle() -> anyhow::Result<()> {
		let start = QuadrupedRunStart::default();
		let mid = start.weight(0.5);
		assert!(mid > 0.35 && mid < 0.65, "smoothstep midpoint, got {mid}");
		Ok(())
	}

	#[test]
	fn weight_clamps_past_one() -> anyhow::Result<()> {
		let start = QuadrupedRunStart::default();
		assert!((start.weight(1.5) - 1.0).abs() < 1e-5);
		Ok(())
	}
}
