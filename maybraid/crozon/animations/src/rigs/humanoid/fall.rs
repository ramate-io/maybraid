use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::Fall;
use crate::rigs::humanoid::apply::{apply_arm, apply_leg};
use crate::{Effects, Animation};

impl<R: HumanoidRig> Animation<R> for Fall<R> {
	fn apply(&self, rig: &mut R) -> Effects {
		apply_leg(rig, Side::Left, 0.0, 0.0);
		apply_leg(rig, Side::Right, 0.0, 0.0);

		for side in [Side::Left, Side::Right] {
			apply_arm(
				rig,
				side,
				0.0,
				self.shoulder_flex(side),
				self.humerus_swing(side),
				0.0,
				self.forearm_flex(),
			);
		}

		Effects::default()
	}
}
