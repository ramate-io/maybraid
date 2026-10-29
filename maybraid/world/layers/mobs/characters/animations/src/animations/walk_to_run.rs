//! One-shot walk-to-run acceleration.
//!
//! Connects [`Walk`](super::Walk) to [`Run`](super::Run), advancing each cycle at its mailbox
//! rate. Progress `0` matches walk at the chosen gait phase; `1` matches run at the handoff phase.

use super::{smoothstep, Run, Walk};
use crate::Progress;

/// Default shift rate: full transition in ~0.25 s at speed `1.0`.
pub const DEFAULT_WALK_TO_RUN_SPEED: f32 = 4.0;
/// Mailbox cadence for the looping run clip.
pub const RUN_CYCLE_SPEED: f32 = 1.68;
/// Mailbox cadence for the looping walk clip.
pub const WALK_CYCLE_SPEED: f32 = 1.08;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkToRun {
	pub walk: Walk,
	pub run: Run,
	/// Shared gait phase at transition entry.
	pub gait_phase: f32,
}

impl Default for WalkToRun {
	fn default() -> Self {
		Self { walk: Walk::default(), run: Run::default(), gait_phase: 0.0 }
	}
}

impl WalkToRun {
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	pub fn gait_phase(&self) -> f32 {
		self.gait_phase.rem_euclid(1.0)
	}

	/// Elapsed wall time for a normalized transition sample.
	pub fn blend_elapsed(progress: f32) -> f32 {
		Progress(progress).clamp() / DEFAULT_WALK_TO_RUN_SPEED
	}

	pub fn walk_phase_at_progress(progress: f32, gait_phase: f32) -> f32 {
		(gait_phase + Self::blend_elapsed(progress) * WALK_CYCLE_SPEED).rem_euclid(1.0)
	}

	pub fn run_phase_at_progress(progress: f32, gait_phase: f32) -> f32 {
		(gait_phase + Self::blend_elapsed(progress) * RUN_CYCLE_SPEED).rem_euclid(1.0)
	}

	pub fn handoff_run_phase(gait_phase: f32) -> f32 {
		Self::run_phase_at_progress(1.0, gait_phase)
	}

	pub fn walk_phase_at_progress_for(&self, progress: f32) -> f32 {
		Self::walk_phase_at_progress(progress, self.gait_phase())
	}

	pub fn run_phase_at_progress_for(&self, progress: f32) -> f32 {
		Self::run_phase_at_progress(progress, self.gait_phase())
	}

	pub fn handoff_run_phase_for(&self) -> f32 {
		Self::handoff_run_phase(self.gait_phase())
	}
}
