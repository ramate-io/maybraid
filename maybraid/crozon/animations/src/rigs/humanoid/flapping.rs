use crozon_rigs::humanoid::HumanoidRig;

use crate::animations::Flapping;
use crate::rigs::humanoid::wing::{apply_flight_body, apply_flight_wings};
use crate::{Animation, Effects};

impl<R: HumanoidRig> Animation<R> for Flapping {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		apply_flight_body(rig);
		apply_flight_wings(rig, self.flap_amount(progress));
		Effects::default()
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
	fn flapping_moves_shoulders() -> anyhow::Result<()> {
		let mut rig = seeded_rig();
		Flapping::default().apply(&mut rig, 0.1);
		let left = rig.pose().get(&Name::from("shoulder.L")).expect("left shoulder");
		let right = rig.pose().get(&Name::from("shoulder.R")).expect("right shoulder");
		// Y swing angles the wing root away from the spine (mirrored).
		assert!(left.swing.abs() > 0.2);
		assert!((left.swing + right.swing).abs() < 1e-4);
		assert!(left.twist.abs() < 1e-5);
		Ok(())
	}
}
