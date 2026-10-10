use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{Idle, Prone, Run, RunStart, Spring, Transition};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl RunStart {
	/// Neutral idle pose at progress zero.
	pub fn idle_pose(&self) -> HumanoidPose {
		self.idle.sample_pose(0.0)
	}

	/// First run contact pose at cycle phase zero.
	pub fn run_pose(&self) -> HumanoidPose {
		self.run.sample_pose(0.0)
	}
}

impl Animation<HumanoidV0Rig> for RunStart {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let weight = self.weight(progress);
		blend_clips(rig, &self.idle, 0.0, &self.run, 0.0, weight);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;
	use character_rigs::authoring::ArmatureOffset;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn start_matches_idle_neutral() -> anyhow::Result<()> {
		let start = RunStart::default();
		let mut start_rig = HumanoidV0Rig::for_clip_test();
		let mut idle_rig = HumanoidV0Rig::for_clip_test();
		start.apply(&mut start_rig, 0.0);
		Idle::default().apply(&mut idle_rig, 0.0);

		for name in start_rig.animation_bone_names() {
			let a = start_rig.rotation(name);
			let b = idle_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_matches_run_phase_zero() -> anyhow::Result<()> {
		let start = RunStart::default();
		let mut start_rig = HumanoidV0Rig::for_clip_test();
		let mut run_rig = HumanoidV0Rig::for_clip_test();
		start.apply(&mut start_rig, 1.0);
		Run::default().apply(&mut run_rig, 0.0);

		for name in start_rig.animation_bone_names() {
			let a = start_rig.rotation(name);
			let b = run_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn mid_launch_flexes_legs_sagittally() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let rest = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut rig, 0.5);

		let posed = rig.character_length("femur.L");
		let rest_dir = rest.character_length("femur.L");
		assert!(posed.z.abs() > rest_dir.z.abs() + 0.05, "launch folds in Z, {posed:?}");
		assert!(posed.x.abs() < 0.08, "launch is not a side swing, {posed:?}");
		Ok(())
	}

	#[test]
	fn end_pose_puts_legs_out_of_phase() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut rig, 1.0);

		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!((left.z - right.z).abs() > 0.05, "legs are out of phase, L={left:?} R={right:?}");
		Ok(())
	}

	#[test]
	fn midpoint_opposes_the_legs() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut rig, 0.5);
		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!(left.z > 0.05 && right.z < -0.05, "mid blend opposes legs L={left:?} R={right:?}");
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let start = RunStart::default();
		start.apply(&mut once, 0.4);
		start.apply(&mut twice, 0.4);
		start.apply(&mut twice, 0.4);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn transition_from_idle_blends_into_mid_launch() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut rig, 0.25);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(RunStart::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.5, 0.5);

		let rest = HumanoidV0Rig::for_clip_test();
		assert!(
			rig.posed_angle("femur.L") > rest.posed_angle("femur.L") + 0.04,
			"blend should move toward run contact"
		);
		let mut full = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut full, 0.5);
		assert!(
			rig.posed_angle("femur.L") < full.posed_angle("femur.L") + 0.02,
			"blend should stay below full launch"
		);
		Ok(())
	}

	#[test]
	fn jump_spring_interrupt_blends_from_visible_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Spring::default().apply(&mut rig, 0.6);
		let from_pose = rig.pose.clone();
		let spring_femur = rig.posed_angle("femur.L");

		let mut blended = HumanoidV0Rig::for_clip_test();
		Transition::from_visible(RunStart::default(), from_pose, ArmatureOffset::IDENTITY)
			.apply(&mut blended, 0.4, 0.25);

		assert!(
			(blended.posed_angle("femur.L") - spring_femur).abs() < 0.12,
			"early blend should keep jump takeoff leg pose visible"
		);
		let mut full_launch = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut full_launch, 0.4);
		assert!(
			blended.posed_angle("femur.L") < full_launch.posed_angle("femur.L") + 0.05,
			"legs should ease toward run launch"
		);
		Ok(())
	}

	#[test]
	fn prone_interrupt_keeps_torso_near_floor_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Prone::default().apply(&mut rig, 1.0);
		let from_pose = rig.pose.clone();
		let prone_root = tip(&rig, "root");

		let mut blended = HumanoidV0Rig::for_clip_test();
		Transition::from_visible(RunStart::default(), from_pose, ArmatureOffset::IDENTITY)
			.apply(&mut blended, 0.0, 0.0);

		let blended_root = tip(&blended, "root");
		assert!(
			(blended_root - prone_root).length() < 0.05,
			"entry should not pop to upright rest"
		);
		Ok(())
	}

	#[test]
	fn longer_femurs_still_match_run_at_end() -> anyhow::Result<()> {
		let mut start_rig = HumanoidV0Rig::for_clip_test();
		let mut run_rig = HumanoidV0Rig::for_clip_test();
		start_rig.seed_rest("femur.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		start_rig.seed_rest("femur.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		start_rig.seed_rest("shin.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));
		start_rig.seed_rest("shin.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));
		run_rig.seed_rest("femur.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		run_rig.seed_rest("femur.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		run_rig.seed_rest("shin.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));
		run_rig.seed_rest("shin.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));

		RunStart::default().apply(&mut start_rig, 1.0);
		Run::default().apply(&mut run_rig, 0.0);

		for name in start_rig.animation_bone_names() {
			let a = start_rig.rotation(name);
			let b = run_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_applies_run_elbow_bend() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut rig, 1.0);
		assert!(rig.posed_angle("forearm.L") > 0.8, "run launch keeps elbow bend");
		Ok(())
	}

	#[test]
	fn midpoint_sits_between_idle_and_run_contact() -> anyhow::Result<()> {
		let mut mid = HumanoidV0Rig::for_clip_test();
		let mut idle = HumanoidV0Rig::for_clip_test();
		let mut run = HumanoidV0Rig::for_clip_test();
		RunStart::default().apply(&mut mid, 0.5);
		Idle::default().apply(&mut idle, 0.0);
		Run::default().apply(&mut run, 0.0);

		assert!(mid.posed_angle("femur.L") > idle.posed_angle("femur.L") + 0.04);
		assert!(mid.posed_angle("femur.L") < run.posed_angle("femur.L") + 0.02);
		Ok(())
	}
}
