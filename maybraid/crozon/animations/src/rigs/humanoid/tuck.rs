use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::Tuck;
use crate::rigs::humanoid::apply::{apply_arm, apply_leg};
use crate::{Animation, Effects};

impl<R: HumanoidRig> Animation<R> for Tuck<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		let femur = self.femur_swing(progress);
		let shin = self.shin_flex(progress);
		apply_leg(rig, Side::Left, femur, shin);
		apply_leg(rig, Side::Right, femur, shin);

		for side in [Side::Left, Side::Right] {
			apply_arm(
				rig,
				side,
				0.0,
				self.shoulder_flex(side, progress),
				self.humerus_swing(side, progress),
				0.0,
				self.forearm_flex(progress),
			);
		}

		Effects::default()
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;

	#[test]
	fn tuck_bends_knees_on_rig() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::<HumanoidV0Rig>::default().apply(&mut rig, 0.5);

		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("shin");
		assert!(shin.flex > 1.0);
		Ok(())
	}

	#[test]
	fn tuck_drives_humerus_swing_not_flex() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::<HumanoidV0Rig>::default().apply(&mut rig, 0.5);

		let humerus = rig.pose().get(&rig.arm(Side::Left).humerus.name).expect("humerus");
		let forearm = rig.pose().get(&rig.arm(Side::Left).forearm.name).expect("forearm");
		assert!(humerus.swing.abs() > 0.1);
		assert!(humerus.flex.abs() < 1e-4);
		assert!(forearm.flex > 0.5);
		Ok(())
	}
}
