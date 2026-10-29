//! One-shot run-to-walk deceleration.
//!
//! Connects [`Run`](super::Run) to [`Walk`](super::Walk), advancing each cycle at its mailbox
//! rate. Progress `0` matches run at the chosen gait phase; `1` matches walk at the handoff phase.

use super::{smoothstep, Run, Walk};
use crate::Progress;

/// Default shift rate: full transition in ~0.25 s at speed `1.0`.
pub const DEFAULT_RUN_TO_WALK_SPEED: f32 = 4.0;
/// Mailbox cadence for the looping run clip.
pub const RUN_CYCLE_SPEED: f32 = 1.68;
/// Mailbox cadence for the looping walk clip.
pub const WALK_CYCLE_SPEED: f32 = 1.08;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunToWalk {
	pub run: Run,
	pub walk: Walk,
	/// Shared gait phase at transition entry.
	pub gait_phase: f32,
}

impl Default for RunToWalk {
	fn default() -> Self {
		Self { run: Run::default(), walk: Walk::default(), gait_phase: 0.0 }
	}
}

impl RunToWalk {
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	pub fn gait_phase(&self) -> f32 {
		self.gait_phase.rem_euclid(1.0)
	}

	pub fn blend_elapsed(progress: f32) -> f32 {
		Progress(progress).clamp() / DEFAULT_RUN_TO_WALK_SPEED
	}

	pub fn run_phase_at_progress(progress: f32, gait_phase: f32) -> f32 {
		(gait_phase + Self::blend_elapsed(progress) * RUN_CYCLE_SPEED).rem_euclid(1.0)
	}

	pub fn walk_phase_at_progress(progress: f32, gait_phase: f32) -> f32 {
		(gait_phase + Self::blend_elapsed(progress) * WALK_CYCLE_SPEED).rem_euclid(1.0)
	}

	pub fn handoff_walk_phase(gait_phase: f32) -> f32 {
		Self::walk_phase_at_progress(1.0, gait_phase)
	}

	pub fn run_phase_at_progress_for(&self, progress: f32) -> f32 {
		Self::run_phase_at_progress(progress, self.gait_phase())
	}

	pub fn walk_phase_at_progress_for(&self, progress: f32) -> f32 {
		Self::walk_phase_at_progress(progress, self.gait_phase())
	}

	pub fn handoff_walk_phase_for(&self) -> f32 {
		Self::handoff_walk_phase(self.gait_phase())
	}
}
