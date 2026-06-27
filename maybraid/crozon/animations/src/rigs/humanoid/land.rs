use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::Land;
use crate::rigs::humanoid::apply::{apply_leg, apply_root};
use crate::{Effects, Animation};

impl<R: HumanoidRig> Animation<R> for Land<R> {
	fn apply(&self, rig: &mut R) -> Effects {
		apply_leg(rig, Side::Left, self.femur_swing(), self.shin_flex());
		apply_leg(rig, Side::Right, self.femur_swing(), self.shin_flex());
		apply_root(rig, self.root_swing());

		Effects::default()
	}
}
