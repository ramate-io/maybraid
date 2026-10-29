//! One-shot quadruped trot-to-still settle.
//!
//! Connects the first diagonal-trot contact pose of [`QuadrupedRun`](super::QuadrupedRun) at
//! cycle phase `0` to neutral [`QuadrupedIdle`](super::QuadrupedIdle). Progress `0` matches run@0;
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
