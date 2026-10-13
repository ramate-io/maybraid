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
		let elbow = self.arm_hold(progress);
		let aim = self.arm_aim_along(progress);
		for side in [Side::Left, Side::Right] {
			let arm = pose.arm_mut(side);
			arm.elbow_flexion = elbow;
			arm.aim = Some(ArmAim { along: aim, roll: 0.0 });
		}
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

	const FOREARM_SEGMENT: f32 = 0.48;
	/// Stylized prone keeps palms above the pelvis plane; lower reads as floor clip.
	const HAND_MIN_Y: f32 = 0.05;
	/// Pre-change held prone elbow joint Z on `for_clip_test()` (main branch).
	const MAIN_SETTLED_FOREARM_L_Z: f32 = 0.699_695_6;

	fn hand_tip(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		let name = match side {
			Side::Left => "forearm.L",
			Side::Right => "forearm.R",
		};
		rig.character_point(name) + rig.character_length(name) * FOREARM_SEGMENT
	}

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn apply_main_branch_prone(rig: &mut HumanoidV0Rig) {
		let prone = Prone::default();
		let mut pose = HumanoidPose::default();
		let femur = prone.femur_swing(1.0);
		let shin = prone.shin_flex(1.0);
		pose.apply_leg(Side::Left, femur, shin);
		pose.apply_leg(Side::Right, femur, shin);
		pose.apply_hip_fold(Side::Left, femur * 0.35);
		pose.apply_hip_fold(Side::Right, femur * 0.35);
		pose.apply_spine_pitch(prone.spine_pitch(1.0));
		let neck = prone.neck_swing(1.0);
		pose.apply_neck_twisted(0.0, 0.0, neck, 0.0, 0.0, neck);
		let hold = prone.arm_hold(1.0);
		pose.apply_arm(Side::Left, 0.0, 0.0, 0.0, hold, hold);
		pose.apply_arm(Side::Right, 0.0, 0.0, 0.0, hold, hold);
		rig.write_pose(&pose);
	}

	fn assert_hands_mirrored(left: Vec3, right: Vec3) -> anyhow::Result<()> {
		if (left.x + right.x).abs() > 0.03 {
			return Err(anyhow::anyhow!("hand X should mirror, left={left:?} right={right:?}"));
		}
		if (left.y - right.y).abs() > 0.03 {
			return Err(anyhow::anyhow!("hand Y should match, left={left:?} right={right:?}"));
		}
		if (left.z - right.z).abs() > 0.03 {
			return Err(anyhow::anyhow!("hand Z should match, left={left:?} right={right:?}"));
		}
		Ok(())
	}

	#[test]
	fn settled_prone_reaches_arms_forward_on_clip_test_rest() -> anyhow::Result<()> {
		let rest = HumanoidV0Rig::for_clip_test();
		let mut rig = HumanoidV0Rig::for_clip_test();
		Prone::default().apply(&mut rig, 1.0);

		let rest_hand = hand_tip(&rest, Side::Left);
		let left = hand_tip(&rig, Side::Left);
		let right = hand_tip(&rig, Side::Right);

		assert_hands_mirrored(left, right)?;

		assert!(
			left.z > rest_hand.z + 1.0,
			"hands should reach far forward (+Z), rest={rest_hand:?} posed={left:?}",
		);
		assert!(
			rig.character_point("forearm.L").z > MAIN_SETTLED_FOREARM_L_Z + 0.25,
			"elbow joint should advance past main held prone, got {}",
			rig.character_point("forearm.L").z,
		);

		let mut main = HumanoidV0Rig::for_clip_test();
		apply_main_branch_prone(&mut main);
		let main_left = hand_tip(&main, Side::Left);
		assert!(
			left.z > main_left.z + 0.35,
			"aimed reach should beat main humerus-only hold, main={main_left:?} new={left:?}",
		);

		assert!(left.y > HAND_MIN_Y && right.y > HAND_MIN_Y, "hands stay above ground clip");
		assert!(
			left.y < rest_hand.y - 0.5,
			"hands settle toward the ground plane, rest_y={} posed_y={}",
			rest_hand.y,
			left.y,
		);

		let forearm_l = rig.posed_angle("forearm.L");
		let forearm_r = rig.posed_angle("forearm.R");
		assert!(
			(forearm_l - forearm_r).abs() < 0.02,
			"symmetric elbow flex, L={forearm_l} R={forearm_r}",
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
