use character_rigs::authoring::{ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Prone;
use crate::rigs::humanoid::apply::{
	apply_arm, apply_hip_fold, apply_leg, apply_neck_twisted, apply_spine_pitch,
};
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for Prone {
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
