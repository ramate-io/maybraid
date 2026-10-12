use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::SquatDescent;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for SquatDescent {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let femur_swing = self.femur_swing(progress);
		let shin_flex = self.shin_flex(progress);

		pose.apply_leg(Side::Left, femur_swing, shin_flex);
		pose.apply_leg(Side::Right, femur_swing, shin_flex);
		let hip = self.hip_fold(progress);
		if hip.abs() > f32::EPSILON {
			pose.apply_hip_fold(Side::Left, hip);
			pose.apply_hip_fold(Side::Right, hip);
		}
		pose.apply_spine_pitch(self.root_swing(progress));
		rig.write_pose(&pose);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;
	use crate::animations::Squat;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn start_matches_upright() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut rig, 0.0);
		let rest = HumanoidV0Rig::for_clip_test();
		for name in ["femur.L", "shin.L", "root"] {
			let posed = tip(&rig, name);
			let rest_tip = tip(&rest, name);
			assert!((posed - rest_tip).length() < 1e-3, "{name} should stay at rest");
		}
		Ok(())
	}

	#[test]
	fn end_matches_held_squat() -> anyhow::Result<()> {
		let mut descent_rig = HumanoidV0Rig::for_clip_test();
		let mut held_rig = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut descent_rig, 1.0);
		Squat::held().apply(&mut held_rig, 1.0);

		for name in descent_rig.animation_bone_names() {
			let a = descent_rig.rotation(name);
			let b = held_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn mid_descent_posed_knee_angle_is_documented() -> anyhow::Result<()> {
		let progress = 0.55;
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut rig, progress);
		let knee_angle = rig.posed_angle("shin.L");
		let hip_angle = rig.posed_angle("femur.L");
		let knee_z = rig.character_point("shin.L").z;
		// Posed-joint anchors for stylization review (for_clip_test rest).
		assert!((knee_angle - 1.297).abs() < 0.02, "shin.L angle, got {knee_angle}");
		assert!((hip_angle - 0.776).abs() < 0.02, "femur.L angle, got {hip_angle}");
		assert!((knee_z - 0.142).abs() < 0.02, "shin.L character Z, got {knee_z}");
		Ok(())
	}

	#[test]
	fn knee_flex_leads_hip_flex_at_mid_descent() -> anyhow::Result<()> {
		let progress = 0.55;
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut rig, progress);
		let knee = rig.posed_angle("shin.L");
		let hip = rig.posed_angle("femur.L");
		let depth = SquatDescent::default().depth(progress);
		let knee_share = (depth / crate::animations::squat_descent::KNEE_LEAD_DEPTH).min(1.0);
		assert!(
			knee > hip * (knee_share / depth) * 0.95,
			"knee should lead hips mid-descent (knee={knee}, hip={hip}, depth={depth})"
		);
		let knee_tip = rig.character_point("shin.L");
		let rest = HumanoidV0Rig::for_clip_test();
		let rest_knee = rest.character_point("shin.L");
		assert!(
			(knee_tip.z - rest_knee.z).abs() > 0.08,
			"knee joint should sit further forward than rest at mid-descent, got {knee_tip:?}"
		);
		Ok(())
	}

	#[test]
	fn both_femurs_fold_sagittally_at_mid_descent() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut rig, 0.5);
		let rest = HumanoidV0Rig::for_clip_test();
		let posed = rig.character_length("femur.L");
		let rest_dir = rest.character_length("femur.L");
		assert!(posed.z.abs() > rest_dir.z.abs() + 0.1, "squat folds in Z, {posed:?}");
		assert!(posed.x.abs() < 0.08, "squat is not a side swing, {posed:?}");
		assert!(
			(rig.posed_angle("femur.L") - rig.posed_angle("femur.R")).abs() < 1e-4,
			"both legs share flexion"
		);
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let descent = SquatDescent::default();
		descent.apply(&mut once, 0.4);
		descent.apply(&mut twice, 0.4);
		descent.apply(&mut twice, 0.4);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn transition_from_idle_blends_into_mid_descent() -> anyhow::Result<()> {
		use crate::animations::{Idle, Squat, Transition};
		use character_rigs::authoring::ArmatureOffset;

		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut rig, 0.0);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(SquatDescent::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.5, 0.5);
		assert!(
			rig.posed_angle("femur.L") > 0.05
				&& rig.posed_angle("femur.L") < Squat::held().femur_peak.abs(),
			"blend should land between idle and full squat"
		);
		Ok(())
	}

	#[test]
	fn unwritten_bones_stay_at_rest() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let rest = HumanoidV0Rig::for_clip_test();
		SquatDescent::default().apply(&mut rig, 0.5);
		for name in ["humerus.L", "humerus.R", "forearm.L", "forearm.R"] {
			let posed = rig.rotation(name);
			let rest_rot = rest.rotation(name);
			assert!(posed.dot(rest_rot).abs() > 1.0 - 1e-5, "{name} should stay at rest");
		}
		Ok(())
	}
}
