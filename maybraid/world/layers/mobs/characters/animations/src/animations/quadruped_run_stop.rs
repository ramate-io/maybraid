//! One-shot quadruped trot-to-still settle.
//!
//! Connects the first diagonal-trot contact pose of [`QuadrupedRun`](super::QuadrupedRun) at cycle
//! phase `0` back to neutral [`QuadrupedIdle`](super::QuadrupedIdle). Progress `0` matches run@0;
//! `1` matches idle@0. Sampling is stateless: progress alone sets the blend weight.

use super::{smoothstep, QuadrupedIdle, QuadrupedRun};
use crate::Progress;

/// Default settle rate: full transition in ~0.28 s at speed `1.0`.
pub const DEFAULT_QUADRUPED_RUN_STOP_SPEED: f32 = 3.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadrupedRunStop {
	pub run: QuadrupedRun,
	pub idle: QuadrupedIdle,
}

impl Default for QuadrupedRunStop {
	fn default() -> Self {
		Self { run: QuadrupedRun::default(), idle: QuadrupedIdle::default() }
	}
}

impl QuadrupedRunStop {
	/// Eased blend weight: 0 at trot contact, 1 at upright rest.
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn weight_endpoints() -> anyhow::Result<()> {
		let stop = QuadrupedRunStop::default();
		assert!(stop.weight(0.0).abs() < 1e-5);
		assert!((stop.weight(1.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn weight_eases_in_the_middle() -> anyhow::Result<()> {
		let stop = QuadrupedRunStop::default();
		let mid = stop.weight(0.5);
		assert!(mid > 0.35 && mid < 0.65, "smoothstep midpoint, got {mid}");
		Ok(())
	}

	#[test]
	fn weight_clamps_past_one() -> anyhow::Result<()> {
		let stop = QuadrupedRunStop::default();
		assert!((stop.weight(1.5) - 1.0).abs() < 1e-5);
		Ok(())
	}
}
