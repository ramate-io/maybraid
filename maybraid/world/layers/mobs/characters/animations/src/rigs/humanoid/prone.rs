use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmAim, ArmatureOffset, HumanoidPose};
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
		let hold = self.arm_hold(progress);
		for side in [Side::Left, Side::Right] {
			pose.apply_arm(side, 0.0, 0.0, 0.0, hold, hold);
			let name = match side {
				Side::Left => "humerus.L",
				Side::Right => "humerus.R",
			};
			if let Some(bone) = rig.binding.definition.id(name) {
				let rest_direction =
					rig.binding.definition.rotation_in_character(&rig.binding.effective_rest, bone)
						* Vec3::Y;
				pose.arm_mut(side).aim =
					Some(ArmAim { along: self.arm_aim_along(progress, rest_direction), roll: 0.0 });
			}
		}
		rig.write_pose(&pose);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		ArmatureOffset::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn settled_prone_pitches_the_spine() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
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

	#[test]
	fn prone_reach_starts_at_bind_and_finishes_forward_down_on_both_sides() -> anyhow::Result<()> {
		let mut start = HumanoidV0Rig::for_clip_test();
		let left_start = start.character_length("humerus.L");
		let right_start = start.character_length("humerus.R");
		Prone::default().apply(&mut start, 0.0);
		assert!(start.character_length("humerus.L").dot(left_start) > 0.999);
		assert!(start.character_length("humerus.R").dot(right_start) > 0.999);

		let mut settled = HumanoidV0Rig::for_clip_test();
		Prone::default().apply(&mut settled, 1.0);
		for side in ["L", "R"] {
			let direction = settled.character_length(&format!("humerus.{side}"));
			assert!(direction.z > 0.85 && direction.y < -0.2, "{side}: {direction:?}");
		}
		Ok(())
	}
}
