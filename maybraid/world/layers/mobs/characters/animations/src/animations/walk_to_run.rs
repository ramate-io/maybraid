//! One-shot walk-to-run acceleration.
//!
//! Connects [`Walk`](super::Walk) to [`Run`](super::Run) while advancing each clip's
//! cycle phase at the mailbox loop rate so a completed shift hands off to run
//! without a phase reset. Progress `0` is full walk at [`WalkToRun::gait_phase`];
//! progress `1` is full run at [`WalkToRun::handoff_run_phase`].
//! Sampling is stateless: progress alone sets blend weight and both phases.

use super::{smoothstep, Run, Walk};
use crate::Progress;

/// Default shift rate: full transition in ~0.25 s at speed `1.0`.
pub const DEFAULT_WALK_TO_RUN_SPEED: f32 = 4.0;

/// Must match [`character_motion::clip::AnimId::Run`] mailbox advance rate.
pub const RUN_CYCLE_SPEED: f32 = 1.68;
/// Must match [`character_motion::clip::AnimId::Walk`] mailbox advance rate.
pub const WALK_CYCLE_SPEED: f32 = 1.08;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkToRun {
	pub walk: Walk,
	pub run: Run,
	/// Shared gait cycle phase for both endpoints (`0..1`).
	pub gait_phase: f32,
}

impl Default for WalkToRun {
	fn default() -> Self {
		Self { walk: Walk::default(), run: Run::default(), gait_phase: 0.0 }
	}
}

impl WalkToRun {
	/// Eased blend weight: 0 at walk, 1 at run.
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	pub fn gait_phase(&self) -> f32 {
		self.gait_phase.rem_euclid(1.0)
	}

	/// Wall-clock elapsed within the shift for normalized `progress`.
	pub fn blend_elapsed(progress: f32) -> f32 {
		Progress(progress).clamp() / DEFAULT_WALK_TO_RUN_SPEED
	}

	/// Walk cycle phase advanced as if the walk loop were playing during the shift.
	pub fn walk_phase_at_progress(progress: f32, gait_phase: f32) -> f32 {
		(gait_phase + Self::blend_elapsed(progress) * WALK_CYCLE_SPEED).rem_euclid(1.0)
	}

	/// Run cycle phase advanced as if the run loop were playing during the shift.
	pub fn run_phase_at_progress(progress: f32, gait_phase: f32) -> f32 {
		(gait_phase + Self::blend_elapsed(progress) * RUN_CYCLE_SPEED).rem_euclid(1.0)
	}

	/// Phase to seed the looping run clip after a completed shift (no reset at handoff).
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn weight_endpoints() -> anyhow::Result<()> {
		let shift = WalkToRun::default();
		assert!(shift.weight(0.0).abs() < 1e-5);
		assert!((shift.weight(1.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn weight_eases_in_the_middle() -> anyhow::Result<()> {
		let shift = WalkToRun::default();
		let mid = shift.weight(0.5);
		assert!(mid > 0.35 && mid < 0.65, "smoothstep midpoint, got {mid}");
		Ok(())
	}

	#[test]
	fn weight_clamps_past_one() -> anyhow::Result<()> {
		let shift = WalkToRun::default();
		assert!((shift.weight(1.5) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn run_phase_tracks_shift_elapsed_time() -> anyhow::Result<()> {
		let gait = 0.25;
		let phase = WalkToRun::handoff_run_phase(gait);
		let expected = (gait + RUN_CYCLE_SPEED / DEFAULT_WALK_TO_RUN_SPEED).rem_euclid(1.0);
		assert!((phase - expected).abs() < 1e-5, "handoff {phase} vs {expected}");
		Ok(())
	}
}
