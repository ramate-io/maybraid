use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Land;
use crate::Animation;

impl Animation<HumanoidV0Rig> for Land {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		pose.apply_leg(Side::Left, self.femur_swing(progress), self.shin_flex(progress));
		pose.apply_leg(Side::Right, self.femur_swing(progress), self.shin_flex(progress));
		pose.apply_root(self.root_swing(progress));

		for side in [Side::Left, Side::Right] {
			pose.apply_arm(
				side,
				0.0,
				self.shoulder_flex(side, progress),
				self.humerus_swing(side, progress),
				self.humerus_flex(side, progress),
				self.forearm_flex(progress),
			);
		}
		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::Animation;

	fn forearm_tip(rig: &HumanoidV0Rig, side: &str) -> Vec3 {
		let bone = format!("forearm.{side}");
		rig.character_point(&bone) + rig.character_length(&bone)
	}

	fn land_peak_progress(land: &Land) -> f32 {
		land.descent_duration() / land.cycle_duration()
	}

	#[test]
	fn land_arms_rest_at_touchdown() {
		let land = Land::default();
		let mut rig = HumanoidV0Rig::for_clip_test();
		land.apply(&mut rig, 0.0);

		assert!(rig.posed_angle("humerus.L") < 0.02);
		assert!(rig.posed_angle("humerus.R") < 0.02);
	}

	#[test]
	fn land_arms_tuck_forward_at_peak_compression() {
		let land = Land::default();
		let peak = land_peak_progress(&land);
		let mut rig = HumanoidV0Rig::for_clip_test();
		land.apply(&mut rig, peak);

		assert!(rig.posed_angle("shin.L") > 0.4, "legs compress at peak");
		let mut rest = HumanoidV0Rig::for_clip_test();
		land.apply(&mut rest, 0.0);
		assert!(
			rig.posed_angle("forearm.L") > rest.posed_angle("forearm.L") + 0.35,
			"elbows bend into guard at peak"
		);
		assert!(
			rig.posed_angle("humerus.L") > rest.posed_angle("humerus.L") + 0.2,
			"humerus leaves T-pose at peak"
		);
		let tip_l = forearm_tip(&rig, "L");
		let rest_l = forearm_tip(&rest, "L");
		assert!(
			tip_l.y < rest_l.y - 0.12,
			"left hand drops with compression, {tip_l:?} vs {rest_l:?}"
		);
	}

	#[test]
	fn land_arm_tuck_is_symmetric_at_peak() {
		let land = Land::default();
		let peak = land_peak_progress(&land);
		let mut rig = HumanoidV0Rig::for_clip_test();
		land.apply(&mut rig, peak);

		let left = rig.posed_angle("forearm.L");
		let right = rig.posed_angle("forearm.R");
		assert!((left - right).abs() < 0.03, "elbow bend matches, L={left} R={right}");
	}
}
