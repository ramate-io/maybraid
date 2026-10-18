use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Fall;
use crate::Animation;

impl Fall {
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		pose.apply_leg(Side::Left, 0.0, 0.0);
		pose.apply_leg(Side::Right, 0.0, 0.0);

		for side in [Side::Left, Side::Right] {
			pose.apply_arm(
				side,
				0.0,
				self.shoulder_flex(side, progress),
				self.humerus_swing(side, progress),
				0.0,
				self.forearm_flex(progress),
			);
		}
		pose
	}
}

impl Animation<HumanoidV0Rig> for Fall {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		rig.write_pose(&self.sample_pose(progress));
	}
}
