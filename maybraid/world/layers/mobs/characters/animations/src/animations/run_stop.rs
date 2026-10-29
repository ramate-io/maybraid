//! One-shot run deceleration through walk into neutral idle.
//!
//! The run-to-walk segment keeps both clips at a shared authored phase; the final segment settles
//! walk to idle at progress zero. Player locomotion may drive progress from measured speed.

use super::{smoothstep, Idle, Run, Walk};
use crate::Progress;

/// Default stop rate for bounded playback and preview.
pub const DEFAULT_RUN_STOP_SPEED: f32 = 2.5;
/// Progress where run-to-walk ends and walk-to-idle begins.
pub const RUN_TO_WALK_END: f32 = 0.55;
/// Planar speed where run-stop yields to low-speed locomotion.
pub const RUN_STOP_HANDOFF_SPEED: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunStop {
	pub run: Run,
	pub walk: Walk,
	pub idle: Idle,
	/// Shared gait phase for run and walk samples.
	pub phase: f32,
	/// When false, omit the run-to-walk segment and start in the walk-to-idle segment.
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
	/// Select the active segment and its smooth 0..1 blend weight.
	pub fn segment(&self, progress: f32) -> RunStopSegment {
		let progress = Progress(progress).clamp();
		if !self.from_run || progress >= RUN_TO_WALK_END {
			let start = if self.from_run { RUN_TO_WALK_END } else { 0.0 };
			let span = (1.0 - start).max(1e-3);
			let local = ((progress - start) / span).clamp(0.0, 1.0);
			RunStopSegment::WalkToIdle(smoothstep(local))
		} else {
			let local = (progress / RUN_TO_WALK_END).clamp(0.0, 1.0);
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

/// Ground speed (m/s) at a run-stop progress sample for a deceleration from `start_speed`.
pub fn speed_for_run_stop_progress(progress: f32, start_speed: f32) -> f32 {
	if start_speed <= RUN_STOP_HANDOFF_SPEED {
		return RUN_STOP_HANDOFF_SPEED;
	}
	let progress = progress.clamp(0.0, RUN_TO_WALK_END);
	let target = 1.0 - progress / RUN_TO_WALK_END;
	let mut low = 0.0_f32;
	let mut high = 1.0_f32;
	for _ in 0..24 {
		let middle = (low + high) * 0.5;
		if smoothstep(middle) < target {
			low = middle;
		} else {
			high = middle;
		}
	}
	let ratio = (low + high) * 0.5;
	RUN_STOP_HANDOFF_SPEED + ratio * (start_speed - RUN_STOP_HANDOFF_SPEED)
}

const IDLE_CYCLE_SPEED: f32 = 0.2;
const WALK_CYCLE_SPEED: f32 = 1.08;

/// Cadence (cycles/s) that keeps posed foot speed near planar speed during run-stop.
pub fn run_stop_cycle_speed(speed: f32, progress: f32) -> f32 {
	match RunStop::default().segment(progress) {
		RunStopSegment::RunToWalk(weight) => {
			let travel = run_stop_foot_travel_per_cycle(weight);
			(speed / travel.max(1e-4)).max(IDLE_CYCLE_SPEED)
		}
		RunStopSegment::WalkToIdle(_) => WALK_CYCLE_SPEED,
	}
}

/// Forward foot travel over one gait cycle at a run-to-walk blend weight.
pub fn run_stop_foot_travel_per_cycle(weight: f32) -> f32 {
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	const SAMPLES: usize = 120;
	let run = Run::default();
	let walk = Walk::default();
	let mut minimum = f32::MAX;
	let mut maximum = f32::MIN;
	for index in 0..SAMPLES {
		let phase = index as f32 / SAMPLES as f32;
		let mut rig = HumanoidV0Rig::for_clip_test();
		crate::rigs::mix::blend_clips(&mut rig, &run, phase, &walk, phase, weight);
		let foot = rig.character_point("shin.L")
			+ rig.character_length("shin.L") * rig.segment_lengths.shin;
		minimum = minimum.min(foot.z);
		maximum = maximum.max(foot.z);
	}
	maximum - minimum
}
