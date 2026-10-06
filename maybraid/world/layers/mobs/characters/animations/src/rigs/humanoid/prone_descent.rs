use bevy::prelude::Vec3;
use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Prone, ProneDescent, Transition};
use crate::rigs::humanoid::apply::{
	apply_arm, apply_hip_fold, apply_leg, apply_neck_twisted, apply_spine_pitch,
};
use crate::{Animation, Effects};
use character_rigs::authoring::ArmatureOffset;

impl Animation<HumanoidV0Rig> for ProneDescent {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let femur = self.femur_swing(progress);
		let shin = self.shin_flex(progress);
		apply_leg(&mut pose, Side::Left, femur, shin);
		apply_leg(&mut pose, Side::Right, femur, shin);
		apply_hip_fold(&mut pose, Side::Left, femur * 0.35);
		apply_hip_fold(&mut pose, Side::Right, femur * 0.35);
		apply_spine_pitch(&mut pose, self.spine_pitch(progress));
		let neck = self.neck_swing(progress);
		apply_neck_twisted(&mut pose, 0.0, 0.0, neck, 0.0, 0.0, neck);
		let hold = self.arm_hold(progress);
		apply_arm(&mut pose, Side::Left, 0.0, 0.0, 0.0, hold, hold);
		apply_arm(&mut pose, Side::Right, 0.0, 0.0, 0.0, hold, hold);
		rig.write_pose(&pose);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::{Idle, Walk};

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn start_matches_upright() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		ProneDescent::default().apply(&mut rig, 0.0);
		let rest = HumanoidV0Rig::for_clip_test();
		for name in ["femur.L", "shin.L", "root", "lumbar"] {
			let posed = tip(&rig, name);
			let rest_tip = tip(&rest, name);
			assert!((posed - rest_tip).length() < 1e-3, "{name} should stay at rest");
		}
		Ok(())
	}

	#[test]
	fn end_matches_held_prone() -> anyhow::Result<()> {
		let mut descent_rig = HumanoidV0Rig::for_clip_test();
		let mut held_rig = HumanoidV0Rig::for_clip_test();
		ProneDescent::default().apply(&mut descent_rig, 1.0);
		Prone::default().apply(&mut held_rig, 1.0);

		for name in descent_rig.animation_bone_names() {
			let a = descent_rig.rotation(name);
			let b = held_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn both_legs_fold_sagittally_at_mid_descent() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		ProneDescent::default().apply(&mut rig, 0.5);
		let rest = HumanoidV0Rig::for_clip_test();
		let posed = rig.character_length("femur.L");
		let rest_dir = rest.character_length("femur.L");
		assert!(posed.z.abs() > rest_dir.z.abs() + 0.05, "prone folds in Z, {posed:?}");
		assert!(posed.x.abs() < 0.08, "prone is not a side swing, {posed:?}");
		assert!(
			(rig.posed_angle("femur.L") - rig.posed_angle("femur.R")).abs() < 1e-4,
			"both legs share flexion"
		);
		Ok(())
	}

	#[test]
	fn spine_pitches_forward_at_mid_descent() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		ProneDescent::default().apply(&mut rig, 0.5);
		let root = tip(&rig, "root");
		assert!(root.z > 0.1, "root should pitch forward at mid descent, got {root:?}");
		assert!(root.x.abs() < 0.08, "root pitch stays sagittal, got {root:?}");
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let descent = ProneDescent::default();
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
	fn transition_from_walk_blends_into_mid_descent() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut rig, 0.25);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(ProneDescent::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.5, 0.5);
		let root = tip(&rig, "root");
		assert!(root.z > 0.05, "blend should pitch the spine toward prone, got {root:?}");
		let rest = HumanoidV0Rig::for_clip_test();
		let rest_root = tip(&rest, "root");
		assert!(
			root.z > rest_root.z + 0.03,
			"blend should leave walk and move toward prone"
		);
		Ok(())
	}

	#[test]
	fn transition_from_idle_blends_into_mid_descent() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut rig, 0.0);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(ProneDescent::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.5, 0.5);
		let root = tip(&rig, "root");
		assert!(
			root.z > 0.05 && root.z < Prone::default().spine_peak,
			"blend should land between idle and full prone"
		);
		Ok(())
	}

	#[test]
	fn jump_interruption_from_mid_descent_stays_deterministic() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		ProneDescent::default().apply(&mut rig, 0.4);
		let interrupted: Vec<_> = rig
			.animation_bone_names()
			.map(|name| (name, rig.rotation(name)))
			.collect();
		ProneDescent::default().apply(&mut rig, 0.4);
		for (name, before) in interrupted {
			let after = rig.rotation(name);
			assert!(after.dot(before).abs() > 1.0 - 1e-5, "re-sample after pause must not drift on {name}");
		}
		Ok(())
	}

	fn scale_leg_rest(rig: &mut HumanoidV0Rig, scale: f32) {
		let mut rest = rig.binding.effective_rest.clone();
		for name in ["femur.L", "femur.R", "shin.L", "shin.R"] {
			if let Some(id) = rig.binding.definition.id(name) {
				rest.local[id.index()].translation *= scale;
			}
		}
		rig.binding.refresh_rest(rest);
		rig.pose.copy_from(&rig.binding.effective_rest);
	}

	#[test]
	fn scaled_legs_match_held_prone_at_endpoint() -> anyhow::Result<()> {
		for scale in [0.75, 1.25] {
			let mut descent_rig = HumanoidV0Rig::for_clip_test();
			let mut held_rig = HumanoidV0Rig::for_clip_test();
			scale_leg_rest(&mut descent_rig, scale);
			scale_leg_rest(&mut held_rig, scale);
			ProneDescent::default().apply(&mut descent_rig, 1.0);
			Prone::default().apply(&mut held_rig, 1.0);
			for name in descent_rig.animation_bone_names() {
				let a = descent_rig.rotation(name);
				let b = held_rig.rotation(name);
				assert!(
					a.dot(b).abs() > 1.0 - 1e-5,
					"scale {scale} endpoint mismatch on {name}"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn unwritten_bones_stay_at_rest() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let rest = HumanoidV0Rig::for_clip_test();
		ProneDescent::default().apply(&mut rig, 0.5);
		for name in ["shoulder.L", "shoulder.R"] {
			let posed = rig.rotation(name);
			let rest_rot = rest.rotation(name);
			assert!(posed.dot(rest_rot).abs() > 1.0 - 1e-5, "{name} should stay at rest");
		}
		Ok(())
	}
}
