use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::Spring;
use crate::rigs::humanoid::apply::{apply_arm, apply_leg, apply_root};
use crate::Animation;

impl<R: HumanoidRig> Animation<R> for Spring<R> {
	fn apply_for(&self, rig: &mut R, progress: f32) {
		apply_leg(rig, Side::Left, self.femur_swing(progress), self.shin_flex(progress));
		apply_leg(rig, Side::Right, self.femur_swing(progress), self.shin_flex(progress));
		apply_root(rig, self.root_swing(progress));

		for side in [Side::Left, Side::Right] {
			apply_arm(
				rig,
				side,
				self.shoulder_swing(progress),
				0.0,
				0.0,
				self.humerus_flex(progress),
				self.forearm_flex(progress),
			);
		}
	}
}
