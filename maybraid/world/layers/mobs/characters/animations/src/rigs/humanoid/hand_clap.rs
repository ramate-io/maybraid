//! Humanoid mapping for [`HandClap`](crate::animations::HandClap).

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::HandClap;
use crate::rigs::humanoid::apply::{apply_neck_twisted, apply_spine_pitch};
use crate::Animation;

impl Animation<HumanoidV0Rig> for HandClap {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		for side in [Side::Left, Side::Right] {
			apply_clap_arm(&mut pose, side, progress, self);
		}
		apply_spine_pitch(&mut pose, self.spine_pitch(progress));
		apply_neck_twisted(&mut pose, 0.0, 0.0, self.neck_nod(progress), 0.0, 0.0, 0.0);
		rig.write_pose(&pose);
	}
}

fn apply_clap_arm(pose: &mut HumanoidPose, side: Side, progress: f32, clap: &HandClap) {
	let amount = clap.clap_amount(progress);
	if amount < 1e-4 {
		return;
	}
	let arm = pose.arm_mut(side);
	arm.shoulder_lift += clap.shoulder_carry(side, progress);
	arm.elbow_flexion += clap.elbow_flex(progress);
	arm.aim = Some(ArmAim {
		along: clap.humerus_along(side, progress),
		roll: clap.humerus_roll(side, progress),
	});
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::Side;

	use super::*;

	const PEAK: f32 = 0.55;

	fn bone_tail(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		let bone = rig.binding.definition.id(name).expect(name);
		let origin = rig.binding.definition.translation_in_character(&rig.pose, bone);
		let along = rig.character_length(name);
		let len = rig.binding.effective_rest.local[bone.index()].translation.length();
		origin + along * len
	}

	fn elbow(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		let name = match side {
			Side::Left => "humerus.L",
			Side::Right => "humerus.R",
		};
		bone_tail(rig, name)
	}

	fn forearm_tail(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		let name = match side {
			Side::Left => "forearm.L",
			Side::Right => "forearm.R",
		};
		bone_tail(rig, name)
	}

	fn posed_at(clap: &HandClap, progress: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut rig, progress);
		rig
	}

	#[test]
	fn hand_clap_contact_hands_in_front_at_chest_height() {
		let clap = HandClap;
		let rig = posed_at(&clap, PEAK);
		let chest = rig.character_point("upper_back");
		for side in [Side::Left, Side::Right] {
			let hand = forearm_tail(&rig, side);
			assert!(
				hand.z > chest.z + 0.45,
				"{side:?} hand in front of chest, hand={hand:?} chest={chest:?}"
			);
			assert!(
				hand.y > chest.y - 0.05 && hand.y < chest.y + 0.75,
				"{side:?} hand near chest height, hand={hand:?} chest={chest:?}"
			);
		}
	}

	#[test]
	fn hand_clap_contact_hands_meet_without_crossing() {
		let clap = HandClap;
		let rig = posed_at(&clap, PEAK);
		let left = forearm_tail(&rig, Side::Left);
		let right = forearm_tail(&rig, Side::Right);
		assert!(left.x >= -0.03, "left stays on +X side of midline, {left:?}");
		assert!(right.x <= 0.03, "right stays on −X side of midline, {right:?}");
		assert!((left.x - right.x).abs() < 0.12, "hands close at midline, {left:?} {right:?}");
	}

	#[test]
	fn hand_clap_contact_mirrors_hand_tails() {
		let clap = HandClap;
		let rig = posed_at(&clap, PEAK);
		let left = forearm_tail(&rig, Side::Left);
		let right = forearm_tail(&rig, Side::Right);
		assert!((left.x + right.x).abs() < 0.02, "mirrored X, {left:?} {right:?}");
		assert!((left.y - right.y).abs() < 0.02, "matched Y, {left:?} {right:?}");
		assert!((left.z - right.z).abs() < 0.02, "matched Z, {left:?} {right:?}");
	}

	#[test]
	fn hand_clap_contact_mirrors_elbows() {
		let clap = HandClap;
		let rig = posed_at(&clap, PEAK);
		let left = elbow(&rig, Side::Left);
		let right = elbow(&rig, Side::Right);
		assert!((left.x + right.x).abs() < 0.02, "mirrored elbows, {left:?} {right:?}");
		assert!((left.y - right.y).abs() < 0.02, "matched elbow height, {left:?} {right:?}");
		assert!((left.z - right.z).abs() < 0.02, "matched elbow depth, {left:?} {right:?}");
	}

	#[test]
	fn hand_clap_brings_forearms_together_at_peak() {
		let clap = HandClap;
		let mut posed = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut posed, PEAK);
		let left = forearm_tail(&posed, Side::Left);
		let right = forearm_tail(&posed, Side::Right);
		let separation = (left - right).length();
		assert!(
			separation < 0.15,
			"forearm tips meet at contact, separation {separation}, {left:?} {right:?}"
		);
	}

	#[test]
	fn hand_clap_moves_forearms_inward_from_rest() {
		let clap = HandClap;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut posed, PEAK);
		let rest_sep =
			(forearm_tail(&rest, Side::Left) - forearm_tail(&rest, Side::Right)).length();
		let posed_sep =
			(forearm_tail(&posed, Side::Left) - forearm_tail(&posed, Side::Right)).length();
		assert!(
			posed_sep < rest_sep - 0.25,
			"clap closes the arms, {posed_sep} vs rest {rest_sep}"
		);
	}

	#[test]
	fn hand_clap_mirrors_forearm_positions() {
		let clap = HandClap;
		let mut posed = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut posed, PEAK);
		let left = forearm_tail(&posed, Side::Left);
		let right = forearm_tail(&posed, Side::Right);
		assert!(
			(left.x + right.x).abs() < 0.12,
			"left/right forearm X should mirror, {left:?} {right:?}"
		);
		assert!((left.y - right.y).abs() < 0.08, "forearm height should match, {left:?} {right:?}");
	}

	#[test]
	fn hand_clap_roll_mirrors_by_side() {
		let clap = HandClap;
		assert!(
			(clap.humerus_roll(Side::Right, PEAK) + clap.humerus_roll(Side::Left, PEAK)).abs()
				< 1e-4
		);
	}

	#[test]
	fn hand_clap_unwritten_bones_match_rest() {
		let clap = HandClap;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut posed, PEAK);
		for name in ["femur.L", "femur.R", "shin.L", "shin.R", "pelvis.L", "pelvis.R"] {
			assert!(posed.posed_angle(name) < 0.02, "unwritten {name} stays at rest");
		}
	}

	#[test]
	fn hand_clap_entry_and_exit_match_rest() {
		let clap = HandClap;
		let rest = HumanoidV0Rig::for_clip_test();
		for progress in [0.0, 1.0] {
			let mut posed = HumanoidV0Rig::for_clip_test();
			clap.apply(&mut posed, progress);
			for side in [Side::Left, Side::Right] {
				let delta = (forearm_tail(&rest, side) - forearm_tail(&posed, side)).length();
				assert!(delta < 0.08, "progress {progress} near rest for {side:?}, delta {delta}");
			}
		}
	}

	#[test]
	fn hand_clap_sampling_does_not_accumulate() {
		let clap = HandClap;
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut once, 0.4);
		clap.apply(&mut twice, 0.4);
		clap.apply(&mut twice, 0.4);
		for side in [Side::Left, Side::Right] {
			let a = forearm_tail(&once, side);
			let b = forearm_tail(&twice, side);
			assert!((a - b).length() < 1e-4, "re-sample at same progress is stable for {side:?}");
		}
	}

	#[test]
	fn hand_clap_interruption_holds_pose_at_progress() {
		let clap = HandClap;
		let mut early = HumanoidV0Rig::for_clip_test();
		let mut late = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut early, 0.12);
		clap.apply(&mut late, 0.55);
		let early_sep =
			(forearm_tail(&early, Side::Left) - forearm_tail(&early, Side::Right)).length();
		let late_sep =
			(forearm_tail(&late, Side::Left) - forearm_tail(&late, Side::Right)).length();
		assert!(
			early_sep > late_sep + 0.05,
			"mid-sweep is wider than clap contact, {early_sep} vs {late_sep}"
		);
	}

	#[test]
	fn hand_clap_entry_exit_pose_matches_rest() {
		let clap = HandClap;
		let rest = HumanoidV0Rig::for_clip_test();
		for progress in [0.0, 1.0] {
			let rig = posed_at(&clap, progress);
			assert_eq!(rig.pose, rest.pose, "progress {progress} leaves full rest pose");
		}
	}

	#[test]
	fn hand_clap_clip_is_quiet_at_loop_boundaries() {
		let clap = HandClap;
		for progress in [0.0, 1.0] {
			assert!(clap.clap_amount(progress) < 1e-4);
			assert!(clap.clap_pulse(progress).abs() < 1e-4);
			assert!(clap.elbow_flex(progress).abs() < 1e-4);
		}
		let mut max_pulse = 0.0_f32;
		for i in 0..100 {
			let t = i as f32 / 99.0;
			if t < 0.30 || t > 0.78 {
				max_pulse = max_pulse.max(clap.clap_pulse(t).abs());
			}
		}
		assert!(max_pulse < 1e-4, "pulse only during hold, max outside {max_pulse}");
	}

	#[test]
	fn hand_clap_transition_from_visible_blends_mid_clap() -> anyhow::Result<()> {
		use crate::animations::Transition;
		use character_rigs::authoring::ArmatureOffset;

		let clap = HandClap;
		let mut rig = HumanoidV0Rig::for_clip_test();
		let from_pose = rig.pose.clone();
		let full = posed_at(&clap, 0.35);
		let transition = Transition::from_visible(clap, from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.35, 0.5);
		let blended = forearm_tail(&rig, Side::Left);
		let rest = forearm_tail(&HumanoidV0Rig::for_clip_test(), Side::Left);
		let target = forearm_tail(&full, Side::Left);
		assert!(
			(blended - rest).length() > 0.05 && (blended - target).length() > 0.05,
			"mid-transition between rest and clap, {blended:?} rest {rest:?} target {target:?}"
		);
		Ok(())
	}

	#[test]
	fn hand_clap_pulses_change_forearm_flex() {
		let clap = HandClap;
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut a, 0.40);
		clap.apply(&mut b, 0.55);
		let angle_a = a.posed_angle("forearm.L");
		let angle_b = b.posed_angle("forearm.L");
		assert!(
			(angle_a - angle_b).abs() > 0.03,
			"pulse should change forearm flex, {angle_a} vs {angle_b}"
		);
	}
}
