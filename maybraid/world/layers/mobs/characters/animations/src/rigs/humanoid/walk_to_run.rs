use bevy::prelude::Vec3;
use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{Prone, Run, Spring, Transition, Walk, WalkToRun};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for WalkToRun {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let weight = self.weight(progress);
		let phase = self.gait_phase();
		blend_clips(rig, &self.walk, phase, &self.run, phase, weight);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn start_matches_walk_at_gait_phase() -> anyhow::Result<()> {
		let shift = WalkToRun { gait_phase: 0.25, ..WalkToRun::default() };
		let phase = shift.gait_phase();
		let mut shift_rig = HumanoidV0Rig::for_clip_test();
		let mut walk_rig = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut shift_rig, 0.0);
		Walk::default().apply(&mut walk_rig, phase);

		for name in shift_rig.animation_bone_names() {
			let a = shift_rig.rotation(name);
			let b = walk_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_matches_run_at_gait_phase() -> anyhow::Result<()> {
		let shift = WalkToRun { gait_phase: 0.25, ..WalkToRun::default() };
		let phase = shift.gait_phase();
		let mut shift_rig = HumanoidV0Rig::for_clip_test();
		let mut run_rig = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut shift_rig, 1.0);
		Run::default().apply(&mut run_rig, phase);

		for name in shift_rig.animation_bone_names() {
			let a = shift_rig.rotation(name);
			let b = run_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn midpoint_sits_between_walk_and_run_at_phase() -> anyhow::Result<()> {
		let shift = WalkToRun::default();
		let phase = shift.gait_phase();
		let mut mid = HumanoidV0Rig::for_clip_test();
		let mut walk = HumanoidV0Rig::for_clip_test();
		let mut run = HumanoidV0Rig::for_clip_test();
		shift.apply(&mut mid, 0.5);
		Walk::default().apply(&mut walk, phase);
		Run::default().apply(&mut run, phase);

		let mid_femur = mid.posed_angle("femur.L");
		let walk_femur = walk.posed_angle("femur.L");
		let run_femur = run.posed_angle("femur.L");
		let lo = walk_femur.min(run_femur) - 0.02;
		let hi = walk_femur.max(run_femur) + 0.02;
		assert!(mid_femur > lo && mid_femur < hi, "mid femur {mid_femur} not in [{lo}, {hi}]");
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let shift = WalkToRun::default();
		shift.apply(&mut once, 0.4);
		shift.apply(&mut twice, 0.4);
		shift.apply(&mut twice, 0.4);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn transition_from_walk_blends_into_mid_shift() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut rig, 0.35);
		let from_pose = rig.pose.clone();
		let walk_femur = rig.posed_angle("femur.L");

		let mut blended = HumanoidV0Rig::for_clip_test();
		Transition::from_visible(WalkToRun::default(), from_pose, ArmatureOffset::IDENTITY)
			.apply(&mut blended, 0.5, 0.5);

		assert!(
			(blended.posed_angle("femur.L") - walk_femur).abs() < 0.2,
			"mid blend should stay near the visible walk pose"
		);
		let mut full = HumanoidV0Rig::for_clip_test();
		WalkToRun::default().apply(&mut full, 0.5);
		assert!(
			blended.posed_angle("femur.L") < full.posed_angle("femur.L") + 0.05,
			"should ease toward run"
		);
		Ok(())
	}

	#[test]
	fn jump_spring_interrupt_blends_from_visible_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Spring::default().apply(&mut rig, 0.55);
		let from_pose = rig.pose.clone();
		let spring_femur = rig.posed_angle("femur.L");

		let mut blended = HumanoidV0Rig::for_clip_test();
		Transition::from_visible(WalkToRun::default(), from_pose, ArmatureOffset::IDENTITY)
			.apply(&mut blended, 0.35, 0.2);

		assert!(
			(blended.posed_angle("femur.L") - spring_femur).abs() < 0.15,
			"early blend should keep jump takeoff leg pose visible"
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
		Transition::from_visible(WalkToRun::default(), from_pose, ArmatureOffset::IDENTITY)
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
		let mut shift_rig = HumanoidV0Rig::for_clip_test();
		let mut run_rig = HumanoidV0Rig::for_clip_test();
		shift_rig.seed_rest("femur.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		shift_rig.seed_rest("femur.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		shift_rig.seed_rest("shin.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));
		shift_rig.seed_rest("shin.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));
		run_rig.seed_rest("femur.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		run_rig.seed_rest("femur.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.8));
		run_rig.seed_rest("shin.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));
		run_rig.seed_rest("shin.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.65));

		WalkToRun::default().apply(&mut shift_rig, 1.0);
		Run::default().apply(&mut run_rig, 0.0);

		for name in shift_rig.animation_bone_names() {
			let a = shift_rig.rotation(name);
			let b = run_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn midpoint_opposes_the_legs() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		WalkToRun::default().apply(&mut rig, 0.5);
		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!(
			left.z > 0.02 && right.z < -0.02,
			"mid blend keeps opposing legs L={left:?} R={right:?}"
		);
		Ok(())
	}
}
