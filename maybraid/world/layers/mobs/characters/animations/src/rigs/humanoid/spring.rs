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

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;
	use crate::animations::Squat;

	const SHOULDER_SWING_BACK: f32 = -0.55;
	const HUMERUS_FLEX_BACK: f32 = 0.65;
	const FOREARM_EXTEND: f32 = -0.35;

	fn forearm_tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		let bone = rig.binding.definition.id(name).expect(name);
		let length = rig.binding.effective_rest.local[bone.index()].translation.y;
		rig.character_point(name) + rig.character_length(name) * length
	}

	fn forearm_tip_side(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		let name = match side {
			Side::Left => "forearm.L",
			Side::Right => "forearm.R",
		};
		forearm_tip(rig, name)
	}

	/// Fight-forward back is −Z; left-arm tips mirror sagittal sign through the bind.
	fn sagittal_tip_z(rig: &HumanoidV0Rig, side: Side) -> f32 {
		let tip = forearm_tip_side(rig, side);
		match side {
			Side::Right => tip.z,
			Side::Left => -tip.z,
		}
	}

	fn apply_leg_synced_arms(rig: &mut HumanoidV0Rig, spring: &Spring, progress: f32) {
		let leg = spring.extend_amount(progress);
		let mut pose = HumanoidPose::default();
		apply_leg(&mut pose, Side::Left, spring.femur_swing(progress), spring.shin_flex(progress));
		apply_leg(&mut pose, Side::Right, spring.femur_swing(progress), spring.shin_flex(progress));
		apply_root(&mut pose, spring.root_swing(progress));
		for side in [Side::Left, Side::Right] {
			apply_arm(
				&mut pose,
				side,
				leg * SHOULDER_SWING_BACK,
				0.0,
				0.0,
				leg * HUMERUS_FLEX_BACK,
				leg * FOREARM_EXTEND,
			);
		}
		rig.write_pose(&pose);
	}

	/// Mid-spring: arms should sit further back than a leg-synced envelope would place them.
	#[test]
	fn spring_arm_backward_reach_leads_knee_extension() {
		let spring = Spring::default();
		let progress = 0.55;
		let mut rig = HumanoidV0Rig::for_clip_test();
		spring.apply(&mut rig, progress);

		let mut squat_rig = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut squat_rig, 1.0);
		let root_dir = squat_rig.character_length("root");
		assert!(root_dir.z > 0.05, "+Z is fight-forward on for_clip_test(), got {root_dir:?}");

		assert!(
			(rig.posed_angle("shoulder.L") - rig.posed_angle("shoulder.R")).abs() < 1e-3,
			"shoulder swing mirrors semantically"
		);
		assert!(
			(rig.posed_angle("humerus.L") - rig.posed_angle("humerus.R")).abs() < 1e-3,
			"humerus flex mirrors semantically"
		);
		assert!(
			(rig.posed_angle("forearm.L") - rig.posed_angle("forearm.R")).abs() < 1e-3,
			"forearm flex mirrors semantically"
		);

		let left_tip = forearm_tip_side(&rig, Side::Left);
		let right_tip = forearm_tip_side(&rig, Side::Right);
		assert!(
			(left_tip.x + right_tip.x).abs() < 0.2,
			"forearm tip X mirrors, L={left_tip:?} R={right_tip:?}"
		);
		assert!(
			(sagittal_tip_z(&rig, Side::Left) - sagittal_tip_z(&rig, Side::Right)).abs() < 0.2,
			"sagittal back depth matches, L={left_tip:?} R={right_tip:?}"
		);

		let knee = rig.posed_angle("shin.L");
		assert!(knee > 0.25, "knees still bent mid-spring, got {knee}");

		let mut synced_rig = HumanoidV0Rig::for_clip_test();
		apply_leg_synced_arms(&mut synced_rig, &spring, progress);
		for side in [Side::Left, Side::Right] {
			let lead_z = sagittal_tip_z(&rig, side);
			let synced_z = sagittal_tip_z(&synced_rig, side);
			assert!(
				lead_z < synced_z - 0.02,
				"{side:?} arm lead reaches further back (−Z), lead={lead_z} synced={synced_z}"
			);
		}
	}

	#[test]
	fn spring_leg_extension_unchanged_by_arm_lead() {
		let spring = Spring::default();
		let leg_remain = 1.0 - spring.extend_amount(0.55);
		assert!((spring.femur_swing(0.55) - spring.squat.femur_peak * leg_remain).abs() < 1e-4);
		assert!((spring.shin_flex(0.55) - spring.squat.shin_peak * leg_remain).abs() < 1e-4);
	}

	#[test]
	fn spring_endpoints_match_main_leg_synced_arms() {
		let spring = Spring::default();
		for &progress in &[0.0, 1.0] {
			let mut lead = HumanoidV0Rig::for_clip_test();
			let mut synced = HumanoidV0Rig::for_clip_test();
			spring.apply(&mut lead, progress);
			apply_leg_synced_arms(&mut synced, &spring, progress);
			for name in lead.animation_bone_names() {
				let a = lead.rotation(name);
				let b = synced.rotation(name);
				assert!(
					a.dot(b).abs() > 1.0 - 1e-5,
					"progress {progress} should match main on {name}"
				);
			}
		}
	}

	#[test]
	fn spring_resampling_does_not_accumulate() {
		let spring = Spring::default();
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		spring.apply(&mut once, 0.55);
		spring.apply(&mut twice, 0.55);
		spring.apply(&mut twice, 0.55);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
	}

	#[test]
	fn spring_arm_amount_is_smooth_at_leg_saturation() {
		let spring = Spring::default();
		let dt = 1.0 / 60.0;
		let mut rig = HumanoidV0Rig::for_clip_test();
		let progress = 0.82;
		spring.apply(&mut rig, progress - dt);
		let before = forearm_tip_side(&rig, Side::Right);
		spring.apply(&mut rig, progress);
		let at = forearm_tip_side(&rig, Side::Right);
		spring.apply(&mut rig, progress + dt);
		let after = forearm_tip_side(&rig, Side::Right);
		let vel_before = (at - before) / dt;
		let vel_after = (after - at) / dt;
		let delta = (vel_after - vel_before).length();
		assert!(delta < 0.05, "tip velocity change at 60 fps should be smooth, got {delta}");
	}
}
