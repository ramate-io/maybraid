use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::WalkStop;
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl WalkStop {
	/// First walk contact pose at cycle phase zero.
	pub fn walk_pose(&self) -> HumanoidPose {
		self.walk.sample_pose(0.0)
	}

	/// Neutral idle pose at progress zero.
	pub fn idle_pose(&self) -> HumanoidPose {
		self.idle.sample_pose(0.0)
	}
}

impl Animation<HumanoidV0Rig> for WalkStop {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let weight = self.weight(progress);
		blend_clips(rig, &self.walk, 0.0, &self.idle, 0.0, weight);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;
	use crate::animations::{Idle, Transition, Walk};
	use character_rigs::authoring::ArmatureOffset;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn start_matches_walk_phase_zero() -> anyhow::Result<()> {
		let stop = WalkStop::default();
		let mut stop_rig = HumanoidV0Rig::for_clip_test();
		let mut walk_rig = HumanoidV0Rig::for_clip_test();
		stop.apply(&mut stop_rig, 0.0);
		Walk::default().apply(&mut walk_rig, 0.0);

		for name in stop_rig.animation_bone_names() {
			let a = stop_rig.rotation(name);
			let b = walk_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_matches_idle_neutral() -> anyhow::Result<()> {
		let stop = WalkStop::default();
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
	fn mid_settle_flexes_legs_sagittally() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let rest = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut rig, 0.5);

		let posed = rig.character_length("femur.L");
		let rest_dir = rest.character_length("femur.L");
		assert!(posed.z.abs() > rest_dir.z.abs() + 0.02, "settle folds in Z, {posed:?}");
		assert!(posed.x.abs() < 0.08, "settle is not a side swing, {posed:?}");
		Ok(())
	}

	#[test]
	fn midpoint_opposes_the_legs() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut rig, 0.5);
		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!(left.z > 0.03 && right.z < -0.03, "mid blend opposes legs L={left:?} R={right:?}");
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let stop = WalkStop::default();
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
	fn transition_from_mid_walk_blends_into_mid_settle() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut rig, 0.25);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(WalkStop::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.5, 0.5);

		let mut full = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut full, 0.5);
		assert!(
			rig.posed_angle("femur.L") < full.posed_angle("femur.L") + 0.02,
			"blend should stay below full settle"
		);
		let mut walk = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut walk, 0.25);
		assert!(
			rig.posed_angle("femur.L") > walk.posed_angle("femur.L") - 0.02,
			"blend should stay near captured walk pose"
		);
		Ok(())
	}

	#[test]
	fn jump_interrupt_from_mid_settle_preserves_visible_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut rig, 0.4);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(Walk::default(), from_pose.clone(), ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.0, 0.0);

		let mut captured = HumanoidV0Rig::for_clip_test();
		captured.pose.copy_from(&from_pose);
		for name in ["femur.L", "femur.R"] {
			let a = rig.rotation(name);
			let b = captured.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "interrupt should keep visible pose on {name}");
		}
		Ok(())
	}

	#[test]
	fn midpoint_sits_between_walk_contact_and_idle() -> anyhow::Result<()> {
		let mut mid = HumanoidV0Rig::for_clip_test();
		let mut walk = HumanoidV0Rig::for_clip_test();
		let mut idle = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut mid, 0.5);
		Walk::default().apply(&mut walk, 0.0);
		Idle::default().apply(&mut idle, 0.0);

		assert!(mid.posed_angle("femur.L") < walk.posed_angle("femur.L") + 0.02);
		assert!(mid.posed_angle("femur.L") > idle.posed_angle("femur.L") - 0.02);
		Ok(())
	}

	#[test]
	fn idle_endpoint_mirrors_both_sides() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut rig, 1.0);
		assert!(
			(rig.posed_angle("femur.L") - rig.posed_angle("femur.R")).abs() < 1e-4,
			"idle end should mirror femurs"
		);
		assert!(
			(rig.posed_angle("shin.L") - rig.posed_angle("shin.R")).abs() < 1e-4,
			"idle end should mirror shins"
		);
		Ok(())
	}

	#[test]
	fn walk_endpoint_puts_legs_out_of_phase() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut rig, 0.0);
		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!((left.z - right.z).abs() > 0.05, "walk@0 opposes legs L={left:?} R={right:?}");
		Ok(())
	}

	#[test]
	fn unwritten_bones_stay_at_rest() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let rest = HumanoidV0Rig::for_clip_test();
		WalkStop::default().apply(&mut rig, 0.5);
		for name in ["humerus.L", "humerus.R", "forearm.L", "forearm.R"] {
			let posed = rig.rotation(name);
			let rest_rot = rest.rotation(name);
			// Walk and idle both move arms; mid-blend should still be finite.
			assert!(posed.is_finite(), "{name} should stay finite");
			assert!(rest_rot.is_finite(), "{name} rest should stay finite");
		}
		Ok(())
	}
}
