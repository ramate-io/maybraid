//! One-shot quadruped still-to-trot launch.
//!
//! Blends from neutral [`QuadrupedIdle`](super::QuadrupedIdle) into [`QuadrupedRun`](super::QuadrupedRun)
//! while advancing the run phase. Progress `1` matches the run loop at its handoff phase.

use super::{smoothstep, QuadrupedIdle, QuadrupedRun};
use crate::Progress;

/// Default launch rate: full transition in ~0.28 s at speed `1.0`.
pub const DEFAULT_QUADRUPED_RUN_START_SPEED: f32 = 3.5;
/// Mailbox cadence for the looping quadruped run clip.
pub const QUADRUPED_RUN_CYCLE_SPEED: f32 = 0.5;

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
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	/// Run cycle phase advanced as if the trot loop were playing during launch.
	pub fn run_phase_at_progress(progress: f32) -> f32 {
		(Progress(progress).clamp() / DEFAULT_QUADRUPED_RUN_START_SPEED * QUADRUPED_RUN_CYCLE_SPEED)
			.rem_euclid(1.0)
	}

	/// Phase to seed the looping run clip after launch.
	pub fn handoff_run_phase() -> f32 {
		Self::run_phase_at_progress(1.0)
	}

	pub fn launch_duration() -> f32 {
		1.0 / DEFAULT_QUADRUPED_RUN_START_SPEED
	}
}
