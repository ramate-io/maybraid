//! One-shot run-to-walk deceleration.
//!
//! Connects [`Run`](super::Run) to [`Walk`](super::Walk) while advancing each clip's
//! cycle phase at the mailbox loop rate so a completed shift hands off to walk
//! without a phase reset. Progress `0` is full run at [`RunToWalk::gait_phase`];
//! progress `1` is full walk at [`RunToWalk::handoff_walk_phase`].
//! Sampling is stateless: progress alone sets blend weight and both phases.

use super::{smoothstep, Run, Walk};
use crate::Progress;

/// Default shift rate: full transition in ~0.25 s at speed `1.0`.
pub const DEFAULT_RUN_TO_WALK_SPEED: f32 = 4.0;

/// Must match [`character_motion::clip::AnimId::Run`] mailbox advance rate.
pub const RUN_CYCLE_SPEED: f32 = 1.68;
/// Must match [`character_motion::clip::AnimId::Walk`] mailbox advance rate.
pub const WALK_CYCLE_SPEED: f32 = 1.08;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunToWalk {
	pub run: Run,
	pub walk: Walk,
	/// Shared gait cycle phase for both endpoints (`0..1`).
	pub gait_phase: f32,
}

impl Default for RunToWalk {
	fn default() -> Self {
		Self { run: Run::default(), walk: Walk::default(), gait_phase: 0.0 }
	}
}

impl RunToWalk {
	/// Eased blend weight: 0 at run, 1 at walk.
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	pub fn gait_phase(&self) -> f32 {
		self.gait_phase.rem_euclid(1.0)
	}

	/// Wall-clock elapsed within the shift for normalized `progress`.
	pub fn blend_elapsed(progress: f32) -> f32 {
		Progress(progress).clamp() / DEFAULT_RUN_TO_WALK_SPEED
	}

	/// Run cycle phase advanced as if the run loop were playing during the shift.
	pub fn run_phase_at_progress(progress: f32, gait_phase: f32) -> f32 {
		(gait_phase + Self::blend_elapsed(progress) * RUN_CYCLE_SPEED).rem_euclid(1.0)
	}

	/// Walk cycle phase advanced as if the walk loop were playing during the shift.
	pub fn walk_phase_at_progress(progress: f32, gait_phase: f32) -> f32 {
		(gait_phase + Self::blend_elapsed(progress) * WALK_CYCLE_SPEED).rem_euclid(1.0)
	}

	/// Phase to seed the looping walk clip after a completed shift (no reset at handoff).
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn weight_endpoints() -> anyhow::Result<()> {
		let shift = RunToWalk::default();
		assert!(shift.weight(0.0).abs() < 1e-5);
		assert!((shift.weight(1.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn weight_eases_in_the_middle() -> anyhow::Result<()> {
		let shift = RunToWalk::default();
		let mid = shift.weight(0.5);
		assert!(mid > 0.35 && mid < 0.65, "smoothstep midpoint, got {mid}");
		Ok(())
	}

	#[test]
	fn weight_clamps_past_one() -> anyhow::Result<()> {
		let shift = RunToWalk::default();
		assert!((shift.weight(1.5) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn walk_phase_tracks_shift_elapsed_time() -> anyhow::Result<()> {
		let gait = 0.25;
		let phase = RunToWalk::handoff_walk_phase(gait);
		let expected = (gait + WALK_CYCLE_SPEED / DEFAULT_RUN_TO_WALK_SPEED).rem_euclid(1.0);
		assert!((phase - expected).abs() < 1e-5, "handoff {phase} vs {expected}");
		Ok(())
	}
}
