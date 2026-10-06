use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Fall;
use crate::rigs::humanoid::apply::{apply_arm, apply_leg};
use crate::Animation;

impl Animation<HumanoidV0Rig> for Fall {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		apply_leg(&mut pose, Side::Left, 0.0, 0.0);
		apply_leg(&mut pose, Side::Right, 0.0, 0.0);

		for side in [Side::Left, Side::Right] {
			apply_arm(
				&mut pose,
				side,
				0.0,
				self.shoulder_flex(side, progress),
				self.humerus_swing(side, progress),
				0.0,
				self.forearm_flex(side, progress),
			);
		}
		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fall_right_shoulder_trails_left_mid_spread() -> anyhow::Result<()> {
		let fall = Fall::default();
		let progress = 0.1;
		let mut rig = HumanoidV0Rig::for_clip_test();
		fall.apply(&mut rig, progress);

		let left = rig.posed_angle("shoulder.L").abs();
		let right = rig.posed_angle("shoulder.R").abs();
		assert!(left > right + 0.05, "left should lead at {progress}, L={left} R={right}");
		Ok(())
	}

	#[test]
	fn fall_both_arms_match_at_full_spread() -> anyhow::Result<()> {
		let fall = Fall::default();
		let mut rig = HumanoidV0Rig::for_clip_test();
		fall.apply(&mut rig, 1.0);
		assert!(
			(rig.posed_angle("shoulder.L") - rig.posed_angle("shoulder.R")).abs() < 1e-3,
			"matched shoulder amplitude at full spread"
		);
		assert!(
			(rig.posed_angle("humerus.L") - rig.posed_angle("humerus.R")).abs() < 1e-3,
			"matched humerus amplitude at full spread"
		);
		Ok(())
	}
}
