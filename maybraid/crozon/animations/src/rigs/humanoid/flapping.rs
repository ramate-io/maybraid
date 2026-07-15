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
		let mut a = seeded_rig();
		let mut b = seeded_rig();
		Flapping::default().apply(&mut a, 0.1);
		Flapping::default().apply(&mut b, 0.1 + 0.5 / Flapping::default().speed);
		let swing_a = a.pose().get(&Name::from("shoulder.L")).expect("left").swing;
		let swing_b = b.pose().get(&Name::from("shoulder.L")).expect("left").swing;
		// Front/back beat lives on swing (Y); half-cycle later the stroke has reversed.
		assert!((swing_a - swing_b).abs() > 0.2);
		assert!(a.pose().get(&Name::from("shoulder.L")).expect("left").flex.abs() < 0.05);
		Ok(())
	}
}
