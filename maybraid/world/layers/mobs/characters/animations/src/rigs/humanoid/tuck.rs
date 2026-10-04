use character_rigs::authoring::{ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Tuck, TuckProfile};
use crate::rigs::humanoid::apply::apply_leg;
use crate::{Animation, Effects};

/// Apply tuck articulation scaled by `amount` in `[0.0, 1.0]`.
pub fn apply_tuck_profile(rig: &mut HumanoidV0Rig, profile: &TuckProfile, amount: f32) -> Effects {
	let mut pose = HumanoidPose::default();
	apply_leg(&mut pose, Side::Left, profile.femur_swing(amount), profile.shin_flex(amount));
	apply_leg(&mut pose, Side::Right, profile.femur_swing(amount), profile.shin_flex(amount));

	for side in [Side::Left, Side::Right] {
		let arm = pose.arm_mut(side);
		arm.shoulder_forward = profile.shoulder_roll(side, amount);
		arm.forward_elevation = profile.humerus_swing(side, amount);
		arm.lateral_elevation = profile.humerus_flex(side, amount);
		arm.axial_rotation = profile.humerus_twist(side, amount);
		arm.elbow_flexion = profile.forearm_flex(amount);
	}

	rig.write_pose(&pose);
	ArmatureOffset::IDENTITY
}

impl Animation<HumanoidV0Rig> for Tuck {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let _ = apply_tuck_profile(rig, &self.profile(), self.tuck_amount(progress));
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
	fn tuck_bends_knees_on_rig() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::default().apply(&mut rig, 0.5);

		let shin = tip(&rig, "shin.L");
		assert!(shin.z > 0.5, "knee flexes toward +Z, got {shin:?}");
		assert!(shin.y < 0.55, "bend passes one radian, got {shin:?}");
		Ok(())
	}

	#[test]
	fn tuck_drives_humerus_lateral_axial_and_forearm() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::default().apply(&mut rig, 0.5);

		let humerus = tip(&rig, "humerus.L");
		let yaw = rig.rotation("humerus.L") * Vec3::Z;
		let forearm = tip(&rig, "forearm.L");
		assert!(humerus.x.abs() > 0.05, "lateral elevation, got {humerus:?}");
		assert!(yaw.x.abs() > 0.05, "axial spin yaws the long axis, got {yaw:?}");
		assert!(forearm.z > 0.05, "elbow flexion reaches +Z, got {forearm:?}");
		Ok(())
	}

	#[test]
	fn tuck_shoulders_roll_inward_symmetrically() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::default().apply(&mut rig, 1.0);

		let left = tip(&rig, "shoulder.L");
		let right = tip(&rig, "shoulder.R");
		assert!(left.z.abs() > 0.05, "shoulder forward flexion, got {left:?}");
		assert!((left.z.abs() - right.z.abs()).abs() < 1e-3, "matched amplitude");
		assert!(left.z.signum() != right.z.signum(), "opposite roll signs");
		assert!(left.x.abs() < 1e-3, "roll stays sagittal, got {left:?}");
		assert!(right.x.abs() < 1e-3, "roll stays sagittal, got {right:?}");
		Ok(())
	}

	#[test]
	fn tuck_drives_humerus_twist_and_forearm_flex() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::default().apply(&mut rig, 1.0);

		let yaw = rig.rotation("humerus.L") * Vec3::Z;
		let forearm = tip(&rig, "forearm.L");
		assert!(yaw.x.abs() > 0.5, "full tuck spins the humerus, got {yaw:?}");
		assert!((forearm - Vec3::Y).length() > 1.0, "full tuck closes the elbow, got {forearm:?}");
		Ok(())
	}
}
