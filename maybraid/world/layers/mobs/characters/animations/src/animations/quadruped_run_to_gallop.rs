//! One-shot quadruped trot-to-gallop acceleration.
//!
//! Connects [`QuadrupedRun`](super::QuadrupedRun) to [`Gallop`](super::Gallop) while advancing
//! each clip's cycle phase at the mailbox loop rate so a completed shift hands off to gallop
//! without a phase reset. Progress `0` is full trot at [`QuadrupedRunToGallop::gait_phase`];
//! progress `1` is full gallop at [`QuadrupedRunToGallop::handoff_gallop_phase`].
//! Sampling is stateless: progress alone sets blend weight and both phases.

use super::{smoothstep, Gallop, QuadrupedRun};
use crate::Progress;

/// Default shift rate: full transition in ~0.25 s at speed `1.0`.
pub const DEFAULT_QUADRUPED_RUN_TO_GALLOP_SPEED: f32 = 4.0;

/// Must match [`character_motion::clip::AnimId::QuadrupedRun`] mailbox advance rate.
pub const QUADRUPED_RUN_CYCLE_SPEED: f32 = 0.5;
/// Must match [`character_motion::clip::AnimId::Gallop`] mailbox advance rate.
pub const GALLOP_CYCLE_SPEED: f32 = 0.35;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadrupedRunToGallop {
	pub run: QuadrupedRun,
	pub gallop: Gallop,
	/// Shared gait cycle phase for both endpoints (`0..1`).
	pub gait_phase: f32,
}

impl Default for QuadrupedRunToGallop {
	fn default() -> Self {
		Self { run: QuadrupedRun::default(), gallop: Gallop::default(), gait_phase: 0.0 }
	}
}

impl QuadrupedRunToGallop {
	/// Eased blend weight: 0 at trot, 1 at gallop.
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	pub fn gait_phase(&self) -> f32 {
		self.gait_phase.rem_euclid(1.0)
	}

	/// Wall-clock elapsed within the shift for normalized `progress`.
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn weight_endpoints() -> anyhow::Result<()> {
		let shift = QuadrupedRunToGallop::default();
		assert!(shift.weight(0.0).abs() < 1e-5);
		assert!((shift.weight(1.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn weight_eases_in_the_middle() -> anyhow::Result<()> {
		let shift = QuadrupedRunToGallop::default();
		let mid = shift.weight(0.5);
		assert!(mid > 0.35 && mid < 0.65, "smoothstep midpoint, got {mid}");
		Ok(())
	}

	#[test]
	fn gallop_phase_tracks_shift_elapsed_time() -> anyhow::Result<()> {
		let gait = 0.25;
		let phase = QuadrupedRunToGallop::handoff_gallop_phase(gait);
		let expected =
			(gait + GALLOP_CYCLE_SPEED / DEFAULT_QUADRUPED_RUN_TO_GALLOP_SPEED).rem_euclid(1.0);
		assert!((phase - expected).abs() < 1e-5, "handoff {phase} vs {expected}");
		Ok(())
	}

	#[test]
	fn run_and_gallop_phases_at_60hz_steps() -> anyhow::Result<()> {
		let gait = 0.1;
		let dt = 1.0 / 60.0;
		let speed = DEFAULT_QUADRUPED_RUN_TO_GALLOP_SPEED;
		let mut progress = 0.0;
		let mut prev_run = QuadrupedRunToGallop::run_phase_at_progress(0.0, gait);
		let mut prev_gallop = QuadrupedRunToGallop::gallop_phase_at_progress(0.0, gait);
		while progress < 1.0 {
			progress = (progress + dt * speed).min(1.0);
			let run = QuadrupedRunToGallop::run_phase_at_progress(progress, gait);
			let gallop = QuadrupedRunToGallop::gallop_phase_at_progress(progress, gait);
			assert!(
				(run - prev_run).abs() < 0.05 || progress >= 1.0,
				"run phase jump at progress {progress}"
			);
			assert!(
				(gallop - prev_gallop).abs() < 0.05 || progress >= 1.0,
				"gallop phase jump at progress {progress}"
			);
			prev_run = run;
			prev_gallop = gallop;
		}
		Ok(())
	}
}
