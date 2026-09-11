//! First-load unveil: wait until spawn terrain is ready and LOD work is quiet.

use crate::flow::GameFlow;
use bevy::prelude::*;
use maybraid_world::{LodJobCounter, WorldSurfaceReady};
use menu_screens::{request_loading_explainer, request_loading_progress};

/// Remaining generate / present / pending-root tickets that still count as
/// "the first wave is finishing." Streaming continues after unveil.
pub const UNVEIL_JOB_THRESHOLD: u64 = 16;
/// Frames the counter must stay at or below [`UNVEIL_JOB_THRESHOLD`] after
/// work has been observed.
pub const UNVEIL_QUIET_FRAMES: u32 = 2;
/// Allow produce to enqueue before treating a still-zero counter as idle.
pub const UNVEIL_ARM_GRACE_SECS: f32 = 1.0;
/// Safety: spawn collider is ready but jobs never quieted.
pub const UNVEIL_TIMEOUT_SECS: f32 = 25.0;

/// Armed on [`GameFlow::LoadingWorld`]. Prevents unveiling on frame 0 when
/// the job counter is still empty because produce has not run.
#[derive(Resource, Debug, Clone)]
pub struct FirstLoadGate {
	pub saw_work: bool,
	pub quiet_frames: u32,
	pub peak: u64,
	pub entered_at: f32,
}

impl FirstLoadGate {
	pub fn new(entered_at: f32) -> Self {
		Self { saw_work: false, quiet_frames: 0, peak: 0, entered_at }
	}

	pub fn observe(&mut self, active: u64) {
		if active > self.peak {
			self.peak = active;
		}
		if active > UNVEIL_JOB_THRESHOLD {
			self.saw_work = true;
			self.quiet_frames = 0;
			return;
		}
		if self.saw_work {
			self.quiet_frames = self.quiet_frames.saturating_add(1);
		}
	}

	pub fn should_unveil(&self, ready: bool, active: u64, now: f32) -> bool {
		if !ready {
			return false;
		}
		let waited = now - self.entered_at;
		if waited >= UNVEIL_TIMEOUT_SECS {
			return true;
		}
		if !self.saw_work {
			return waited >= UNVEIL_ARM_GRACE_SECS && active <= UNVEIL_JOB_THRESHOLD;
		}
		active <= UNVEIL_JOB_THRESHOLD && self.quiet_frames >= UNVEIL_QUIET_FRAMES
	}

	pub fn progress(&self, ready: bool, active: u64) -> f32 {
		let from_jobs = if self.peak == 0 {
			if self.saw_work {
				0.55
			} else {
				0.08
			}
		} else {
			(1.0 - (active as f32 / self.peak as f32)).clamp(0.08, 0.95)
		};
		if ready && self.saw_work && active <= UNVEIL_JOB_THRESHOLD {
			return from_jobs.max(0.9);
		}
		if ready {
			from_jobs.max(0.45)
		} else {
			from_jobs.min(0.4)
		}
	}

	pub fn explainer(&self, ready: bool, active: u64) -> &'static str {
		if !ready {
			return "Waiting for the ground…";
		}
		if self.saw_work && active > UNVEIL_JOB_THRESHOLD {
			return "Streaming the world…";
		}
		"Almost ready…"
	}
}

pub(crate) fn arm_first_load(mut commands: Commands, time: Res<Time>) {
	commands.insert_resource(FirstLoadGate::new(time.elapsed_secs()));
}

pub(crate) fn disarm_first_load(mut commands: Commands) {
	commands.remove_resource::<FirstLoadGate>();
}

pub(crate) fn finish_world_loading(
	mut commands: Commands,
	ready: Res<WorldSurfaceReady>,
	jobs: Option<Res<LodJobCounter>>,
	mut gate: Option<ResMut<FirstLoadGate>>,
	time: Res<Time>,
	mut flow: ResMut<NextState<GameFlow>>,
) {
	let active = jobs.as_deref().map(LodJobCounter::active).unwrap_or(0);
	let Some(gate) = gate.as_deref_mut() else {
		if ready.0 {
			flow.set(GameFlow::World);
		}
		return;
	};
	gate.observe(active);
	request_loading_progress(&mut commands, gate.progress(ready.0, active));
	request_loading_explainer(&mut commands, gate.explainer(ready.0, active));
	if gate.should_unveil(ready.0, active, time.elapsed_secs()) {
		flow.set(GameFlow::World);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn gate_at(entered_at: f32) -> FirstLoadGate {
		FirstLoadGate::new(entered_at)
	}

	#[test]
	fn idle_zero_does_not_unveil_before_grace() {
		let gate = gate_at(0.0);
		assert!(!gate.should_unveil(true, 0, 0.2));
		assert!(!gate.should_unveil(true, 0, UNVEIL_ARM_GRACE_SECS - 0.01));
	}

	#[test]
	fn idle_after_grace_unveils_when_ready() {
		let gate = gate_at(0.0);
		assert!(gate.should_unveil(true, 0, UNVEIL_ARM_GRACE_SECS));
		assert!(!gate.should_unveil(false, 0, UNVEIL_ARM_GRACE_SECS));
	}

	#[test]
	fn busy_counter_waits_for_quiet_frames() {
		let mut gate = gate_at(0.0);
		gate.observe(80);
		assert!(gate.saw_work);
		assert!(!gate.should_unveil(true, 80, 2.0));
		gate.observe(8);
		assert!(!gate.should_unveil(true, 8, 2.0));
		gate.observe(8);
		assert!(gate.should_unveil(true, 8, 2.0));
	}

	#[test]
	fn timeout_unveils_only_when_ready() {
		let mut gate = gate_at(0.0);
		gate.observe(400);
		assert!(!gate.should_unveil(false, 400, UNVEIL_TIMEOUT_SECS));
		assert!(gate.should_unveil(true, 400, UNVEIL_TIMEOUT_SECS));
	}

	#[test]
	fn work_spike_resets_quiet_frames() {
		let mut gate = gate_at(0.0);
		gate.observe(80);
		gate.observe(4);
		gate.observe(80);
		assert_eq!(gate.quiet_frames, 0);
		assert!(!gate.should_unveil(true, 80, 5.0));
	}
}
