//! One-shot run-to-walk-to-still deceleration.
//!
//! Connects [`Run`](super::Run) at a shared gait phase through [`Walk`](super::Walk) into
//! neutral [`Idle`](super::Idle). Progress `0` matches run@`phase`; `1` matches idle@0.
//! When `from_run` is false the run→walk band is skipped (walk-speed coast only).

use super::{smoothstep, Idle, Run, Walk};
use crate::Progress;

/// Default stop rate: full transition in ~0.4 s at speed `1.0`.
pub const DEFAULT_RUN_STOP_SPEED: f32 = 2.5;
/// Progress where the run→walk band ends and walk→idle begins.
pub const RUN_TO_WALK_END: f32 = 0.55;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunStop {
	pub run: Run,
	pub walk: Walk,
	pub idle: Idle,
	/// Shared gait phase for run and walk samples.
	pub phase: f32,
	/// When false, playback starts at [`RUN_TO_WALK_END`] (walk→idle only).
	pub from_run: bool,
}

impl Default for RunStop {
	fn default() -> Self {
		Self {
			run: Run::default(),
			walk: Walk::default(),
			idle: Idle::default(),
			phase: 0.0,
			from_run: true,
		}
	}
}

impl RunStop {
	/// Map outer progress into the active segment and return a 0..1 blend weight.
	pub fn segment(&self, progress: f32) -> RunStopSegment {
		let p = Progress(progress).clamp();
		if !self.from_run || p >= RUN_TO_WALK_END {
			let start = if self.from_run { RUN_TO_WALK_END } else { 0.0 };
			let span = (1.0 - start).max(1e-3);
			let local = ((p - start) / span).clamp(0.0, 1.0);
			RunStopSegment::WalkToIdle(smoothstep(local))
		} else {
			let local = (p / RUN_TO_WALK_END).clamp(0.0, 1.0);
			RunStopSegment::RunToWalk(smoothstep(local))
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RunStopSegment {
	RunToWalk(f32),
	WalkToIdle(f32),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn segment_endpoints_from_run() -> anyhow::Result<()> {
		let stop = RunStop::default();
		assert!(matches!(stop.segment(0.0), RunStopSegment::RunToWalk(w) if w < 1e-5));
		assert!(matches!(stop.segment(1.0), RunStopSegment::WalkToIdle(w) if (w - 1.0).abs() < 1e-5));
		Ok(())
	}

	#[test]
	fn walk_coast_skips_run_band() -> anyhow::Result<()> {
		let stop = RunStop { from_run: false, ..RunStop::default() };
		assert!(matches!(stop.segment(0.0), RunStopSegment::WalkToIdle(_)));
		assert!(matches!(stop.segment(0.25), RunStopSegment::WalkToIdle(_)));
		Ok(())
	}

	#[test]
	fn run_to_walk_band_is_limited() -> anyhow::Result<()> {
		let stop = RunStop::default();
		assert!(matches!(stop.segment(RUN_TO_WALK_END - 0.01), RunStopSegment::RunToWalk(_)));
		assert!(matches!(stop.segment(RUN_TO_WALK_END + 0.01), RunStopSegment::WalkToIdle(_)));
		Ok(())
	}
}
