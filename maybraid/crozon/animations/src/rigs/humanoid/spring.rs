use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::Spring;
use crate::rigs::humanoid::apply::{apply_arm, apply_leg, apply_root};
use crate::{Effects, Animation};

impl<R: HumanoidRig> Animation<R> for Spring<R> {
	fn apply(&self, rig: &mut R) -> Effects {
		apply_leg(rig, Side::Left, self.femur_swing(), self.shin_flex());
		apply_leg(rig, Side::Right, self.femur_swing(), self.shin_flex());
		apply_root(rig, self.root_swing());

		for side in [Side::Left, Side::Right] {
			apply_arm(
				rig,
				side,
				self.shoulder_swing(),
				0.0,
				0.0,
				self.humerus_flex(),
				self.forearm_flex(),
			);
		}

		Effects::default()
	}
}
