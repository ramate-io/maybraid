//! One-shot run-to-walk deceleration (walk→idle is [`Approach`](super::Approach) in #1022).
//!
//! Connects [`Run`](super::Run) at a shared gait phase through [`Walk`](super::Walk).
//! Progress `0` matches run@`phase`; [`RUN_TO_WALK_END`] matches walk@`phase`.
//! Player locomotion drives progress from measured planar speed, not wall-clock time.
//! When `from_run` is false the run→walk band is skipped (playground walk→idle only).

use super::{smoothstep, Idle, Run, Walk};
use crate::Progress;

/// Default stop rate kept for playground scrubbing; player uses speed-driven progress.
pub const DEFAULT_RUN_STOP_SPEED: f32 = 2.5;
/// Progress where the run→walk band ends. Player locomotion stops here and hands off.
pub const RUN_TO_WALK_END: f32 = 0.55;
/// Planar speed (m/s) where run-stop yields to low-speed locomotion (walk / approach).
pub const RUN_STOP_HANDOFF_SPEED: f32 = 1.0;

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

/// Map planar speed during deceleration to run-stop progress in `[0, RUN_TO_WALK_END]`.
pub fn run_stop_progress_from_speed(speed: f32, start_speed: f32) -> f32 {
	if start_speed <= RUN_STOP_HANDOFF_SPEED {
		return RUN_TO_WALK_END;
	}
	let span = (start_speed - RUN_STOP_HANDOFF_SPEED).max(1e-3);
	let t = ((speed - RUN_STOP_HANDOFF_SPEED) / span).clamp(0.0, 1.0);
	RUN_TO_WALK_END * (1.0 - smoothstep(t))
}

/// Ground speed (m/s) for a run-stop progress sample along a decel from `start_speed`.
pub fn speed_for_run_stop_progress(progress: f32, start_speed: f32) -> f32 {
	if start_speed <= RUN_STOP_HANDOFF_SPEED {
		return RUN_STOP_HANDOFF_SPEED;
	}
	let p = progress.clamp(0.0, RUN_TO_WALK_END);
	let t = 1.0 - p / RUN_TO_WALK_END;
	let mut lo = 0.0_f32;
	let mut hi = 1.0_f32;
	for _ in 0..24 {
		let mid = (lo + hi) * 0.5;
		if smoothstep(mid) < t {
			lo = mid;
		} else {
			hi = mid;
		}
	}
	let ratio = (lo + hi) * 0.5;
	RUN_STOP_HANDOFF_SPEED + ratio * (start_speed - RUN_STOP_HANDOFF_SPEED)
}

const IDLE_CYCLE_SPEED: f32 = 0.2;
const WALK_CYCLE_SPEED: f32 = 1.08;

/// Cadence (cycles/s) that keeps posed foot speed near planar speed during run-stop.
pub fn run_stop_cycle_speed(speed: f32, progress: f32) -> f32 {
	let segment = RunStop::default().segment(progress);
	match segment {
		RunStopSegment::RunToWalk(weight) => {
			let travel = run_stop_foot_travel_per_cycle(weight);
			(speed / travel.max(1e-4)).max(IDLE_CYCLE_SPEED)
		}
		RunStopSegment::WalkToIdle(_) => WALK_CYCLE_SPEED,
	}
}

/// Forward foot travel over one gait cycle at a run→walk blend weight.
pub fn run_stop_foot_travel_per_cycle(weight: f32) -> f32 {
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	let run = Run::default();
	let walk = Walk::default();
	let samples = 120;
	let mut min_z = f32::MAX;
	let mut max_z = f32::MIN;
	for i in 0..samples {
		let phase = i as f32 / samples as f32;
		let mut rig = HumanoidV0Rig::for_clip_test();
		crate::rigs::mix::blend_clips(&mut rig, &run, phase, &walk, phase, weight);
		let tip = rig.character_point("shin.L") + rig.character_length("shin.L");
		min_z = min_z.min(tip.z);
		max_z = max_z.max(tip.z);
	}
	max_z - min_z
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn segment_endpoints_from_run() -> anyhow::Result<()> {
		let stop = RunStop::default();
		assert!(matches!(stop.segment(0.0), RunStopSegment::RunToWalk(w) if w < 1e-5));
		assert!(
			matches!(stop.segment(1.0), RunStopSegment::WalkToIdle(w) if (w - 1.0).abs() < 1e-5)
		);
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

	#[test]
	fn progress_tracks_speed() {
		let start = 10.5;
		assert!((run_stop_progress_from_speed(start, start)).abs() < 1e-5);
		assert!(
			(run_stop_progress_from_speed(RUN_STOP_HANDOFF_SPEED, start) - RUN_TO_WALK_END).abs()
				< 1e-5
		);
		let mid = run_stop_progress_from_speed(5.0, start);
		assert!(mid > 0.0 && mid < RUN_TO_WALK_END);
	}

	#[test]
	fn run_stop_foot_cadence_tracks_ground_speed() -> anyhow::Result<()> {
		let start = 10.5;
		for fraction in [0.25, 0.5, 0.75] {
			let progress = RUN_TO_WALK_END * fraction;
			let ground = speed_for_run_stop_progress(progress, start);
			let segment = RunStop::default().segment(progress);
			let weight = match segment {
				RunStopSegment::RunToWalk(w) => w,
				_ => panic!("expected run→walk segment at {progress}"),
			};
			let travel = run_stop_foot_travel_per_cycle(weight);
			let cadence = run_stop_cycle_speed(ground, progress);
			let foot_speed = travel * cadence;
			let mismatch = (foot_speed - ground).abs() / ground.max(1e-3);
			eprintln!(
				"run-stop {:.0}%: travel {travel:.3} m/cycle x cadence {cadence:.3} = foot {foot_speed:.3} m/s vs ground {ground:.3} m/s (mismatch {:.1}%)",
				fraction * 100.0,
				mismatch * 100.0
			);
			assert!(
				mismatch < 0.08,
				"fraction {fraction}: foot {foot_speed:.3} m/s vs ground {ground:.3} m/s"
			);
		}
		Ok(())
	}
}
