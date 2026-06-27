use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::Land;
use crate::rigs::humanoid::apply::{apply_leg, apply_root};
use crate::{Animation, Effects};

impl<R: HumanoidRig> Animation<R> for Land<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		apply_leg(rig, Side::Left, self.femur_swing(progress), self.shin_flex(progress));
		apply_leg(rig, Side::Right, self.femur_swing(progress), self.shin_flex(progress));
		apply_root(rig, self.root_swing(progress));

		Effects::default()
	}
}
