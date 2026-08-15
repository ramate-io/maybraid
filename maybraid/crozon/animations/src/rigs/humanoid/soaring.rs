use crozon_rigs::humanoid::HumanoidRig;

use crate::animations::Soaring;
use crate::rigs::humanoid::wing::{apply_flight_body, apply_flight_wings};
use crate::Animation;

impl<R: HumanoidRig> Animation<R> for Soaring {
	fn apply_for(&self, rig: &mut R, progress: f32) {
		apply_flight_body(rig);
		apply_flight_wings(rig, self.flap_amount(progress));
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use crozon_rigs::{BonePose, Name};

	use super::*;
	use crate::Animation;

	fn seeded_rig() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::imported();
		for bone in [
			"root",
			"shoulder.L",
			"shoulder.R",
			"humerus.L",
			"humerus.R",
			"forearm.L",
			"forearm.R",
			"femur.L",
			"femur.R",
			"shin.L",
			"shin.R",
		] {
			rig.pose_mut()
				.insert(BonePose::new(Name::from(bone), bevy::prelude::Transform::IDENTITY));
		}
		rig
	}

	#[test]
	fn soaring_holds_spread_while_gliding() -> anyhow::Result<()> {
		let soar = Soaring::default();
		let mut rig = seeded_rig();
		let glide_t = soar.burst_duration() + soar.pause * 0.5;
		soar.apply(&mut rig, glide_t);
		let left = rig.pose().get(&Name::from("shoulder.L")).expect("left shoulder");
		let right = rig.pose().get(&Name::from("shoulder.R")).expect("right shoulder");
		assert!(left.swing.abs() > 0.2);
		assert!((left.swing + right.swing).abs() < 1e-4);
		// Glide hold stays near T-pose on Z — no big overhead lift.
		assert!(left.flex.abs() < 0.05);
		assert!(left.twist.abs() < 1e-5);
		Ok(())
	}
}
