use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Idle, Transition, Walk, WalkStart};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl WalkStart {
	/// Neutral idle pose at progress zero.
	pub fn idle_pose(&self) -> HumanoidPose {
		self.idle.sample_pose(0.0)
	}

	/// First walk contact pose at cycle phase zero.
	pub fn walk_pose(&self) -> HumanoidPose {
		self.walk.sample_pose(0.0)
	}
}

impl Animation<HumanoidV0Rig> for WalkStart {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let weight = self.weight(progress);
		blend_clips(rig, &self.idle, 0.0, &self.walk, 0.0, weight);
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
		let start = WalkStart::default();
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
	fn end_matches_walk_phase_zero() -> anyhow::Result<()> {
		let start = WalkStart::default();
		let mut start_rig = HumanoidV0Rig::for_clip_test();
		let mut walk_rig = HumanoidV0Rig::for_clip_test();
		start.apply(&mut start_rig, 1.0);
		Walk::default().apply(&mut walk_rig, 0.0);

		for name in start_rig.animation_bone_names() {
			let a = start_rig.rotation(name);
			let b = walk_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn mid_step_off_flexes_legs_sagittally() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let rest = HumanoidV0Rig::for_clip_test();
		WalkStart::default().apply(&mut rig, 0.5);

		let posed = rig.character_length("femur.L");
		let rest_dir = rest.character_length("femur.L");
		assert!(posed.z.abs() > rest_dir.z.abs() + 0.03, "step-off folds in Z, {posed:?}");
		assert!(posed.x.abs() < 0.08, "step-off is not a side swing, {posed:?}");
		Ok(())
	}

	#[test]
	fn end_pose_puts_legs_out_of_phase() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		WalkStart::default().apply(&mut rig, 1.0);

		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!((left.z - right.z).abs() > 0.05, "legs are out of phase, L={left:?} R={right:?}");
		Ok(())
	}

	#[test]
	fn midpoint_opposes_the_legs() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		WalkStart::default().apply(&mut rig, 0.5);
		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!(left.z > 0.03 && right.z < -0.03, "mid blend opposes legs L={left:?} R={right:?}");
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let start = WalkStart::default();
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
	fn transition_from_idle_blends_into_mid_step_off() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut rig, 0.25);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(WalkStart::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.5, 0.5);

		let rest = HumanoidV0Rig::for_clip_test();
		assert!(
			rig.posed_angle("femur.L") > rest.posed_angle("femur.L") + 0.02,
			"blend should move toward walk contact"
		);
		let mut full = HumanoidV0Rig::for_clip_test();
		WalkStart::default().apply(&mut full, 0.5);
		assert!(
			rig.posed_angle("femur.L") < full.posed_angle("femur.L") + 0.02,
			"blend should stay below full step-off"
		);
		Ok(())
	}

	#[test]
	fn end_applies_forward_torso_lean() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		WalkStart::default().apply(&mut rig, 1.0);

		let root = tip(&rig, "root");
		assert!(root.z > 0.05, "forward lean goes to +Z, got {root:?}");
		assert!(root.x.abs() < 1e-3, "lean must not yaw, got {root:?}");
		Ok(())
	}

	#[test]
	fn midpoint_sits_between_idle_and_walk_contact() -> anyhow::Result<()> {
		let mut mid = HumanoidV0Rig::for_clip_test();
		let mut idle = HumanoidV0Rig::for_clip_test();
		let mut walk = HumanoidV0Rig::for_clip_test();
		WalkStart::default().apply(&mut mid, 0.5);
		Idle::default().apply(&mut idle, 0.0);
		Walk::default().apply(&mut walk, 0.0);

		assert!(mid.posed_angle("femur.L") > idle.posed_angle("femur.L") + 0.02);
		assert!(mid.posed_angle("femur.L") < walk.posed_angle("femur.L") + 0.02);
		Ok(())
	}
}
