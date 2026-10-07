use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Spring;
use crate::rigs::humanoid::apply::{apply_arm, apply_leg, apply_root};
use crate::Animation;

impl Animation<HumanoidV0Rig> for Spring {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		apply_leg(&mut pose, Side::Left, self.femur_swing(progress), self.shin_flex(progress));
		apply_leg(&mut pose, Side::Right, self.femur_swing(progress), self.shin_flex(progress));
		apply_root(&mut pose, self.root_swing(progress));

		for side in [Side::Left, Side::Right] {
			apply_arm(
				&mut pose,
				side,
				self.shoulder_swing(progress),
				0.0,
				0.0,
				self.humerus_flex(progress),
				self.forearm_flex(progress),
			);
		}
		rig.write_pose(&pose);
	}
}

fn forearm_tip_z(rig: &HumanoidV0Rig, side: Side) -> f32 {
	let name = match side {
		Side::Left => "forearm.L",
		Side::Right => "forearm.R",
	};
	(rig.character_point(name) + rig.character_length(name) * 0.25).z
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Mid-spring: arms should sit further back than a leg-synced envelope would place them.
	#[test]
	fn spring_arm_backward_reach_leads_knee_extension() {
		let spring = Spring::default();
		let progress = 0.55;
		let mut rig = HumanoidV0Rig::for_clip_test();
		spring.apply(&mut rig, progress);

		let tip_z = forearm_tip_z(&rig, Side::Right);
		let knee = rig.posed_angle("shin.L");
		let leg = spring.extend_amount(progress);
		let synced_shoulder = leg * (-0.55);
		let actual = spring.shoulder_swing(progress);
		assert!(
			actual < synced_shoulder - 0.02,
			"shoulder should lead, actual={actual} synced={synced_shoulder}"
		);
		assert!(knee > 0.25, "knees still bent mid-spring, got {knee}");
		assert!(
			tip_z < -0.42,
			"forearm reaches further back with arm lead, got {tip_z} (was ~-0.382)"
		);
	}

	#[test]
	fn spring_leg_extension_unchanged_by_arm_lead() {
		let spring = Spring::default();
		let leg_remain = 1.0 - spring.extend_amount(0.55);
		assert!((spring.femur_swing(0.55) - spring.squat.femur_peak * leg_remain).abs() < 1e-4);
		assert!((spring.shin_flex(0.55) - spring.squat.shin_peak * leg_remain).abs() < 1e-4);
	}
}
