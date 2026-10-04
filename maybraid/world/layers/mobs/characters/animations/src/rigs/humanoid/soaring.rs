use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::Soaring;
use crate::rigs::humanoid::wing::{apply_flight_body, apply_flight_wings};
use crate::Animation;

impl Animation<HumanoidV0Rig> for Soaring {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		apply_flight_body(&mut pose);
		apply_flight_wings(&mut pose, self.flap_amount(progress));
		rig.write_pose(&pose);
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
	fn soaring_holds_spread_while_gliding() -> anyhow::Result<()> {
		let soar = Soaring::default();
		let mut rig = HumanoidV0Rig::imported();
		let glide_t = soar.burst_duration() + soar.pause * 0.5;
		soar.apply(&mut rig, glide_t);
		let left = tip(&rig, "shoulder.L");
		let right = tip(&rig, "shoulder.R");
		assert!(left.z.abs() > 0.15, "held stroke is sagittal, got {left:?}");
		assert!((left.z + right.z).abs() < 1e-3, "opposite shoulder bias, got {left:?} {right:?}");
		assert!(left.x.abs() < 0.05, "glide hold stays near T-pose laterally, got {left:?}");
		let yaw = rig.rotation("shoulder.L") * Vec3::Z;
		assert!(yaw.x.abs() < 1e-3, "no shoulder twist, got {yaw:?}");
		Ok(())
	}
}
