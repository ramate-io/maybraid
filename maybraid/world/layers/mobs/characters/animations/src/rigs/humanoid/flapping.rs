use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::Flapping;
use crate::rigs::humanoid::wing::{apply_flight_body, apply_flight_wings};
use crate::Animation;

impl Animation<HumanoidV0Rig> for Flapping {
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
	fn flapping_moves_shoulders() -> anyhow::Result<()> {
		let mut a = HumanoidV0Rig::imported();
		let mut b = HumanoidV0Rig::imported();
		Flapping::default().apply(&mut a, 0.1);
		Flapping::default().apply(&mut b, 0.1 + 0.5 / Flapping::default().speed);
		let swing_a = tip(&a, "shoulder.L");
		let swing_b = tip(&b, "shoulder.L");
		// Sagittal stroke; half a cycle later the shoulder has reversed.
		assert!((swing_a.z - swing_b.z).abs() > 0.15, "a {swing_a:?} b {swing_b:?}");
		assert!(swing_a.x.abs() < 0.05, "stroke is not a lateral lift, got {swing_a:?}");
		Ok(())
	}
}
