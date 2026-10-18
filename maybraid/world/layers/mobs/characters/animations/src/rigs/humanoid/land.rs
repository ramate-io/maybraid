use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Land;
use crate::Animation;

impl Land {
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		pose.apply_leg(Side::Left, self.femur_swing(progress), self.shin_flex(progress));
		pose.apply_leg(Side::Right, self.femur_swing(progress), self.shin_flex(progress));
		pose.apply_root(self.root_swing(progress));
		pose
	}
}

impl Animation<HumanoidV0Rig> for Land {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		rig.write_pose(&self.sample_pose(progress));
	}
}
