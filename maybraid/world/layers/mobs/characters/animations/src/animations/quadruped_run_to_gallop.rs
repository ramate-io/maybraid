//! One-shot quadruped trot-to-gallop acceleration.
//!
//! Connects [`QuadrupedRun`](super::QuadrupedRun) to [`Gallop`](super::Gallop) while advancing
//! both loop phases at their mailbox rates. Progress `1` matches gallop at the handoff phase.

use super::{smoothstep, Gallop, QuadrupedRun};
use crate::Progress;

/// Default shift rate: full transition in ~0.25 s at speed `1.0`.
pub const DEFAULT_QUADRUPED_RUN_TO_GALLOP_SPEED: f32 = 4.0;
/// Mailbox cadence for the looping quadruped run clip.
pub const QUADRUPED_RUN_CYCLE_SPEED: f32 = 0.5;
/// Mailbox cadence for the looping gallop clip.
pub const GALLOP_CYCLE_SPEED: f32 = 0.35;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadrupedRunToGallop {
	pub run: QuadrupedRun,
	pub gallop: Gallop,
	/// Shared gait phase at transition entry.
	pub gait_phase: f32,
}

impl Default for QuadrupedRunToGallop {
	fn default() -> Self {
		Self { run: QuadrupedRun::default(), gallop: Gallop::default(), gait_phase: 0.0 }
	}
}

impl QuadrupedRunToGallop {
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	pub fn gait_phase(&self) -> f32 {
		self.gait_phase.rem_euclid(1.0)
	}

	pub fn blend_elapsed(progress: f32) -> f32 {
		Progress(progress).clamp() / DEFAULT_QUADRUPED_RUN_TO_GALLOP_SPEED
	}

	pub fn run_phase_at_progress(progress: f32, gait_phase: f32) -> f32 {
		(gait_phase + Self::blend_elapsed(progress) * QUADRUPED_RUN_CYCLE_SPEED).rem_euclid(1.0)
	}

	pub fn gallop_phase_at_progress(progress: f32, gait_phase: f32) -> f32 {
		(gait_phase + Self::blend_elapsed(progress) * GALLOP_CYCLE_SPEED).rem_euclid(1.0)
	}

	pub fn handoff_gallop_phase(gait_phase: f32) -> f32 {
		Self::gallop_phase_at_progress(1.0, gait_phase)
	}

	pub fn run_phase_at_progress_for(&self, progress: f32) -> f32 {
		Self::run_phase_at_progress(progress, self.gait_phase())
	}

	pub fn gallop_phase_at_progress_for(&self, progress: f32) -> f32 {
		Self::gallop_phase_at_progress(progress, self.gait_phase())
	}

	pub fn handoff_gallop_phase_for(&self) -> f32 {
		Self::handoff_gallop_phase(self.gait_phase())
	}
}
