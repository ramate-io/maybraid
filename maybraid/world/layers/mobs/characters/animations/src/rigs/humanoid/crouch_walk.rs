use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{CrouchedWalk, Squat};
use crate::{Animation, Progress};

impl Animation<HumanoidV0Rig> for CrouchedWalk {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let squat = Squat::held();
		let depth = self.depth.clamp(0.0, 1.0);
		for side in [Side::Left, Side::Right] {
			pose.apply_leg(side, squat.femur_swing(depth), squat.shin_flex(depth));
			pose.apply_hip_fold(side, squat.hip_fold(depth));
		}
		pose.apply_spine_pitch(squat.root_swing(depth));

		let weight = self.leg_cycle.clamp(0.0, 1.0);
		if weight > f32::EPSILON {
			let gait = self.walk.sample_pose(Progress(progress).cycle());
			for side in [Side::Left, Side::Right] {
				let gait_leg = gait.legs[side.index()];
				let leg = pose.leg_mut(side);
				leg.hip_flexion += gait_leg.hip_flexion * weight;
				leg.hip_abduction += gait_leg.hip_abduction * weight;
				leg.hip_rotation += gait_leg.hip_rotation * weight;
				leg.knee_flexion += gait_leg.knee_flexion * weight;
				leg.pelvis_flexion += gait_leg.pelvis_flexion * weight;
				leg.pelvis_lateral += gait_leg.pelvis_lateral * weight;
				leg.pelvis_turn += gait_leg.pelvis_turn * weight;
			}
			for (arm, gait_arm) in pose.arms.iter_mut().zip(gait.arms) {
				let influence = weight * 0.35;
				arm.forward_elevation += gait_arm.forward_elevation * influence;
				arm.lateral_elevation += gait_arm.lateral_elevation * influence;
				arm.axial_rotation += gait_arm.axial_rotation * influence;
				arm.elbow_flexion += gait_arm.elbow_flexion * influence;
				arm.shoulder_forward += gait_arm.shoulder_forward * influence;
				arm.shoulder_lift += gait_arm.shoulder_lift * influence;
				arm.shoulder_twist += gait_arm.shoulder_twist * influence;
			}
		}
		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Vec3;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn zero_cycle_matches_held_squat_and_has_identity_root_effects() -> anyhow::Result<()> {
		let mut expected = HumanoidV0Rig::for_clip_test();
		let mut actual = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut expected, 1.0);
		let effects = CrouchedWalk::blended(1.0, 0.0).apply(&mut actual, 0.37);
		if !effects.is_identity() {
			anyhow::bail!("crouch walk should leave root displacement to the caller");
		}
		for name in ["femur.L", "femur.R", "shin.L", "shin.R", "root", "lumbar"] {
			if expected.rotation(name).dot(actual.rotation(name)).abs() < 1.0 - 1e-5 {
				anyhow::bail!("zero leg-cycle pose differs from held squat at {name}");
			}
		}
		Ok(())
	}

	#[test]
	fn leg_cycle_moves_actual_femur_pose_without_unfolding_spine() -> anyhow::Result<()> {
		let crouch = CrouchedWalk::blended(1.0, 0.35);
		let mut early = HumanoidV0Rig::for_clip_test();
		let mut later = HumanoidV0Rig::for_clip_test();
		let mut held = HumanoidV0Rig::for_clip_test();
		crouch.apply(&mut early, 0.0);
		crouch.apply(&mut later, 0.5);
		Squat::held().apply(&mut held, 1.0);
		if (tip(&early, "femur.L") - tip(&later, "femur.L")).length() < 0.05 {
			anyhow::bail!("gait phase must move the composed left femur");
		}
		for name in ["root", "lumbar", "midback", "upper_back"] {
			if early.rotation(name).dot(held.rotation(name)).abs() < 1.0 - 1e-5 {
				anyhow::bail!("crouch gait should preserve the held folded spine at {name}");
			}
		}
		Ok(())
	}
}
