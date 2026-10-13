use character_rigs::authoring::{ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Prone;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for Prone {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let femur = self.femur_swing(progress);
		let shin = self.shin_flex(progress);
		pose.apply_leg(Side::Left, femur, shin);
		pose.apply_leg(Side::Right, femur, shin);
		pose.apply_hip_fold(Side::Left, femur * 0.35);
		pose.apply_hip_fold(Side::Right, femur * 0.35);
		pose.apply_spine_pitch(self.spine_pitch(progress));
		let neck = self.neck_swing(progress);
		pose.apply_neck_twisted(0.0, 0.0, neck, 0.0, 0.0, neck);
		let reach = self.arm_reach(progress);
		let humerus_forward = self.arm_humerus_forward(progress);
		let humerus_lateral = self.arm_humerus_lateral(progress);
		pose.apply_arm(
			Side::Left,
			reach,
			0.0,
			humerus_forward,
			humerus_lateral,
			self.arm_elbow_flex(Side::Left, progress),
		);
		pose.apply_arm(
			Side::Right,
			reach,
			0.0,
			humerus_forward,
			humerus_lateral,
			self.arm_elbow_flex(Side::Right, progress),
		);
		rig.write_pose(&pose);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		ArmatureOffset::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn settled_prone_reaches_arms_forward_on_imported_rest() -> anyhow::Result<()> {
		let rest = HumanoidV0Rig::for_clip_test();
		let mut rig = HumanoidV0Rig::for_clip_test();
		Prone::default().apply(&mut rig, 1.0);
		let rest_z = rest.character_point("forearm.L").z;
		let posed_z = rig.character_point("forearm.L").z;
		assert!(
			posed_z > rest_z + 0.08,
			"prone should reach forward (+Z), rest={rest_z:.4} posed={posed_z:.4}",
		);
		assert!(
			rig.posed_angle("shoulder.L") > 0.12,
			"shoulder should drive the reach, got {}",
			rig.posed_angle("shoulder.L"),
		);
		assert!(
			(rig.posed_angle("forearm.L") - rig.posed_angle("forearm.R")).abs() > 0.03,
			"elbows should be mildly asymmetric",
		);
		Ok(())
	}

	#[test]
	fn settled_prone_pitches_the_spine() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let prone = Prone::default();
		let effects = prone.apply(&mut rig, 1.0);
		if !effects.is_identity() {
			return Err(anyhow::anyhow!("held prone must not offset the armature"));
		}
		let root = tip(&rig, "root");
		if root.z < 0.3 {
			return Err(anyhow::anyhow!("root should pitch forward, got {root:?}"));
		}
		if root.x.abs() > 1e-3 {
			return Err(anyhow::anyhow!("root pitch must stay sagittal, got {root:?}"));
		}
		let lumbar = tip(&rig, "lumbar");
		if lumbar.z < 0.2 {
			return Err(anyhow::anyhow!("lumbar should share the fold, got {lumbar:?}"));
		}
		Ok(())
	}
}
