//! One-shot quadruped still-to-trot launch.
//!
//! Blends from the mailbox's visible still pose into [`QuadrupedRun`](super::QuadrupedRun) while
//! advancing run cycle phase with launch progress. Progress `0` keeps the entry pose; `1` matches
//! run at [`Self::handoff_run_phase`] for a seamless loop handoff.

use super::{smoothstep, QuadrupedIdle, QuadrupedRun};
use crate::Progress;

/// Default launch rate: full transition in ~0.28 s at speed `1.0`.
pub const DEFAULT_QUADRUPED_RUN_START_SPEED: f32 = 3.5;

/// Must match [`character_motion::clip::AnimId::QuadrupedRun`] mailbox advance rate.
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
	/// Eased blend weight: 0 at idle neutral, 1 at run phase zero.
	pub fn weight(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	/// Run cycle phase advanced as if the trot loop were playing during the launch.
	///
	/// [`DEFAULT_QUADRUPED_RUN_START_SPEED`] maps progress to wall time; multiply by
	/// [`QUADRUPED_RUN_CYCLE_SPEED`] to stay aligned with [`AnimClip::quadruped_run`].
	pub fn run_phase_at_progress(progress: f32) -> f32 {
		let clamped = Progress(progress).clamp();
		let elapsed = clamped / DEFAULT_QUADRUPED_RUN_START_SPEED;
		(elapsed * QUADRUPED_RUN_CYCLE_SPEED).rem_euclid(1.0)
	}

	/// Phase to seed the looping run clip after a completed launch (no reset at handoff).
	pub fn handoff_run_phase() -> f32 {
		Self::run_phase_at_progress(1.0)
	}

	/// Wall-clock duration of a full launch at default rate (seconds).
	pub fn launch_duration() -> f32 {
		1.0 / DEFAULT_QUADRUPED_RUN_START_SPEED
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

	#[test]
	fn run_phase_tracks_launch_elapsed_time() -> anyhow::Result<()> {
		let phase = QuadrupedRunStart::run_phase_at_progress(1.0);
		let expected = QuadrupedRunStart::launch_duration() * QUADRUPED_RUN_CYCLE_SPEED;
		assert!((phase - expected.rem_euclid(1.0)).abs() < 1e-5);
		Ok(())
	}
}
