use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Fall;
use crate::Animation;

impl Animation<HumanoidV0Rig> for Fall {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
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
				self.forearm_flex(side, progress),
			);
		}
		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;

	use super::*;

	#[test]
	fn composed_fall_pose_keeps_left_lead_through_the_longer_ramp() -> anyhow::Result<()> {
		let fall = Fall::default();
		let mut early = HumanoidV0Rig::for_clip_test();
		let effects = fall.apply(&mut early, 0.20);
		let left = early.posed_angle("humerus.L");
		let right = early.posed_angle("humerus.R");
		assert!(left > right + 0.05, "left arm should lead, L={left} R={right}");
		assert!(effects.is_identity());
		assert!(fall.shoulder_flex(Side::Left, 0.20) * fall.shoulder_flex(Side::Right, 0.20) < 0.0);
		Ok(())
	}

	#[test]
	fn spread_reaches_full_after_delayed_ramp() -> anyhow::Result<()> {
		let fall = Fall::default();
		assert!(fall.spread_amount(Side::Left, 0.35) > 0.999);
		assert!(fall.spread_amount(Side::Right, 0.35) < 1.0);
		assert_eq!(fall.spread_amount(Side::Right, 0.40), 1.0);
		Ok(())
	}
}
