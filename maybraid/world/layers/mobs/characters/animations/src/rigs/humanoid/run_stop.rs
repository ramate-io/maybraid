use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{RunStop, RunStopSegment};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for RunStop {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let phase = self.phase.fract();
		match self.segment(progress) {
			RunStopSegment::RunToWalk(weight) => {
				blend_clips(rig, &self.run, phase, &self.walk, phase, weight);
			}
			RunStopSegment::WalkToIdle(weight) => {
				blend_clips(rig, &self.walk, phase, &self.idle, 0.0, weight);
			}
		}
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;

	use super::*;
	use crate::animations::{Idle, Run, Transition, Walk, RUN_TO_WALK_END};

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn start_matches_run_at_phase() -> anyhow::Result<()> {
		let phase = 0.35;
		let stop = RunStop { phase, ..RunStop::default() };
		let mut stop_rig = HumanoidV0Rig::for_clip_test();
		let mut run_rig = HumanoidV0Rig::for_clip_test();
		stop.apply(&mut stop_rig, 0.0);
		Run::default().apply(&mut run_rig, phase);

		for name in stop_rig.animation_bone_names() {
			let a = stop_rig.rotation(name);
			let b = run_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_matches_idle_neutral() -> anyhow::Result<()> {
		let stop = RunStop::default();
		let mut stop_rig = HumanoidV0Rig::for_clip_test();
		let mut idle_rig = HumanoidV0Rig::for_clip_test();
		stop.apply(&mut stop_rig, 1.0);
		Idle::default().apply(&mut idle_rig, 0.0);

		for name in stop_rig.animation_bone_names() {
			let a = stop_rig.rotation(name);
			let b = idle_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn run_to_walk_midpoint_opposes_legs() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		RunStop::default().apply(&mut rig, RUN_TO_WALK_END * 0.5);
		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!(left.z > 0.02 && right.z < -0.02, "legs oppose L={left:?} R={right:?}");
		Ok(())
	}

	#[test]
	fn walk_to_idle_midpoint_between_walk_and_idle() -> anyhow::Result<()> {
		let phase = 0.35;
		let stop = RunStop { phase, ..RunStop::default() };
		let mut mid = HumanoidV0Rig::for_clip_test();
		let mut walk = HumanoidV0Rig::for_clip_test();
		let mut idle = HumanoidV0Rig::for_clip_test();
		stop.apply(&mut mid, RUN_TO_WALK_END + (1.0 - RUN_TO_WALK_END) * 0.5);
		Walk::default().apply(&mut walk, phase);
		Idle::default().apply(&mut idle, 0.0);

		let mid_femur = mid.posed_angle("femur.L");
		assert!(
			mid_femur.abs() < walk.posed_angle("femur.L").abs() + 0.02,
			"mid stop is closer to rest than full walk"
		);
		assert!(
			mid_femur.abs() > idle.posed_angle("femur.L").abs() + 0.01,
			"mid stop still carries walk flexion"
		);
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let stop = RunStop::default();
		stop.apply(&mut once, 0.4);
		stop.apply(&mut twice, 0.4);
		stop.apply(&mut twice, 0.4);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn transition_from_run_blends_into_mid_stop() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut rig, 0.35);
		let from_pose = rig.pose.clone();
		let transition = Transition::from_visible(
			RunStop { phase: 0.35, ..RunStop::default() },
			from_pose,
			ArmatureOffset::IDENTITY,
		);
		transition.apply(&mut rig, 0.5, 0.5);

		let mut full = HumanoidV0Rig::for_clip_test();
		RunStop::default().apply(&mut full, 0.5);
		assert!(
			rig.posed_angle("femur.L") < full.posed_angle("femur.L") + 0.02,
			"interrupted blend stays below full stop pose"
		);
		Ok(())
	}

	#[test]
	fn walk_coast_start_matches_walk_phase() -> anyhow::Result<()> {
		let phase = 0.42;
		let stop = RunStop { phase, from_run: false, ..RunStop::default() };
		let mut stop_rig = HumanoidV0Rig::for_clip_test();
		let mut walk_rig = HumanoidV0Rig::for_clip_test();
		stop.apply(&mut stop_rig, 0.0);
		Walk::default().apply(&mut walk_rig, phase);

		for name in stop_rig.animation_bone_names() {
			let a = stop_rig.rotation(name);
			let b = walk_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_applies_neutral_torso() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		RunStop::default().apply(&mut rig, 1.0);
		let root = tip(&rig, "root");
		assert!(root.z.abs() < 0.04, "idle rest is near neutral, got {root:?}");
		Ok(())
	}

	fn pose_delta(a: &HumanoidV0Rig, b: &HumanoidV0Rig, name: &str) -> f32 {
		(1.0 - a.rotation(name).dot(b.rotation(name)).abs()).max(0.0)
	}

	fn assert_pose_continuous(a: &HumanoidV0Rig, b: &HumanoidV0Rig, label: &str) {
		for name in a.animation_bone_names() {
			let delta = pose_delta(a, b, name);
			assert!(delta < 0.02, "{label}: rotation jump on {name} (delta {delta})");
		}
	}

	#[test]
	fn interrupt_to_walk_preserves_pose() -> anyhow::Result<()> {
		let phase = 0.35;
		let progress = 0.3;
		let stop = RunStop { phase, ..RunStop::default() };
		let mut stop_rig = HumanoidV0Rig::for_clip_test();
		stop.apply(&mut stop_rig, progress);
		let from_pose = stop_rig.pose.clone();

		let transition =
			Transition::from_visible(Walk::default(), from_pose, ArmatureOffset::IDENTITY);
		let mut blended = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut blended, 0.5, phase);

		let mut walk = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut walk, phase);
		assert_pose_continuous(&blended, &walk, "interrupt to walk");
		Ok(())
	}

	#[test]
	fn interrupt_to_run_preserves_pose() -> anyhow::Result<()> {
		let phase = 0.35;
		let progress = 0.2;
		let stop = RunStop { phase, ..RunStop::default() };
		let mut stop_rig = HumanoidV0Rig::for_clip_test();
		stop.apply(&mut stop_rig, progress);
		let from_pose = stop_rig.pose.clone();

		let transition =
			Transition::from_visible(Run::default(), from_pose, ArmatureOffset::IDENTITY);
		let mut blended = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut blended, 0.5, phase);

		let mut run = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut run, phase);
		assert_pose_continuous(&blended, &run, "interrupt to run");
		Ok(())
	}

	#[test]
	fn interrupt_to_jump_starts_from_stop_pose() -> anyhow::Result<()> {
		use crate::animations::TwoFootedJump;

		let phase = 0.35;
		let progress = 0.25;
		let stop = RunStop { phase, ..RunStop::default() };
		let mut stop_rig = HumanoidV0Rig::for_clip_test();
		stop.apply(&mut stop_rig, progress);
		let from_pose = stop_rig.pose.clone();

		let jump = TwoFootedJump::default();
		let transition = Transition::from_visible(jump, from_pose, ArmatureOffset::IDENTITY);
		let mut at_switch = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut at_switch, 0.0, 0.0);
		assert_pose_continuous(&at_switch, &stop_rig, "jump switch onset");

		let mut early = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut early, 0.05, 0.0);
		assert_pose_continuous(&early, &stop_rig, "jump early blend");
		Ok(())
	}

	#[test]
	fn interrupt_to_squat_descent_starts_from_stop_pose() -> anyhow::Result<()> {
		use crate::animations::SquatDescent;

		let phase = 0.35;
		let progress = 0.25;
		let stop = RunStop { phase, ..RunStop::default() };
		let mut stop_rig = HumanoidV0Rig::for_clip_test();
		stop.apply(&mut stop_rig, progress);
		let from_pose = stop_rig.pose.clone();

		let descent = SquatDescent::default();
		let transition = Transition::from_visible(descent, from_pose, ArmatureOffset::IDENTITY);
		let mut at_switch = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut at_switch, 0.0, 0.0);
		assert_pose_continuous(&at_switch, &stop_rig, "squat switch onset");

		let mut early = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut early, 0.05, 0.0);
		assert_pose_continuous(&early, &stop_rig, "squat early blend");
		Ok(())
	}

	#[test]
	fn gait_phase_advances_during_stop() -> anyhow::Result<()> {
		let phase = 0.35;
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		let stop = RunStop { phase, ..RunStop::default() };
		stop.apply(&mut a, 0.2);
		stop.apply(&mut b, 0.2);
		let stop2 = RunStop { phase: phase + 0.1, ..RunStop::default() };
		stop2.apply(&mut b, 0.2);
		assert!(pose_delta(&a, &b, "femur.L") > 1e-4, "advancing phase should change posed joints");
		Ok(())
	}
}
