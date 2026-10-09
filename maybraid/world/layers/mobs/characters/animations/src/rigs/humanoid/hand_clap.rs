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
		apply_neck_twisted(
			&mut pose,
			0.0,
			0.0,
			self.neck_nod(progress),
			0.0,
			0.0,
			0.0,
		);
		rig.write_pose(&pose);
	}
}

fn apply_clap_arm(pose: &mut HumanoidPose, side: Side, progress: f32, clap: &HandClap) {
	let amount = clap.clap_amount(progress);
	if amount < 1e-4 {
		return;
	}
	let arm = pose.arm_mut(side);
	arm.shoulder_forward += clap.shoulder_carry(progress);
	arm.elbow_flexion += clap.elbow_flex(progress);
	arm.aim = Some(ArmAim {
		along: clap.humerus_along(side, progress),
		roll: clap.humerus_roll(side, progress),
	});
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;

	const PEAK: f32 = 0.55;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.character_point(name)
	}

	#[test]
	fn hand_clap_brings_forearms_together_at_peak() {
		let clap = HandClap;
		let mut posed = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut posed, PEAK);
		let left = tip(&posed, "forearm.L");
		let right = tip(&posed, "forearm.R");
		let separation = (left - right).length();
		assert!(
			separation < 0.35,
			"forearms meet near the chest, separation {separation}, {left:?} {right:?}"
		);
	}

	#[test]
	fn hand_clap_moves_forearms_inward_from_rest() {
		let clap = HandClap;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut posed, PEAK);
		let rest_sep = (tip(&rest, "forearm.L") - tip(&rest, "forearm.R")).length();
		let posed_sep = (tip(&posed, "forearm.L") - tip(&posed, "forearm.R")).length();
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
		let left = tip(&posed, "forearm.L");
		let right = tip(&posed, "forearm.R");
		assert!(
			(left.x + right.x).abs() < 0.12,
			"left/right forearm X should mirror, {left:?} {right:?}"
		);
		assert!(
			(left.y - right.y).abs() < 0.08,
			"forearm height should match, {left:?} {right:?}"
		);
	}

	#[test]
	fn hand_clap_roll_mirrors_by_side() {
		let clap = HandClap;
		assert!(
			(clap.humerus_roll(Side::Right, PEAK) + clap.humerus_roll(Side::Left, PEAK)).abs() < 1e-4
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
			for bone in ["forearm.L", "forearm.R"] {
				let delta = (tip(&rest, bone) - tip(&posed, bone)).length();
				assert!(delta < 0.08, "progress {progress} near rest for {bone}, delta {delta}");
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
		for bone in ["forearm.L", "forearm.R"] {
			let a = tip(&once, bone);
			let b = tip(&twice, bone);
			assert!((a - b).length() < 1e-4, "re-sample at same progress is stable for {bone}");
		}
	}

	#[test]
	fn hand_clap_interruption_holds_pose_at_progress() {
		let clap = HandClap;
		let mut early = HumanoidV0Rig::for_clip_test();
		let mut late = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut early, 0.12);
		clap.apply(&mut late, 0.55);
		let early_sep = (tip(&early, "forearm.L") - tip(&early, "forearm.R")).length();
		let late_sep = (tip(&late, "forearm.L") - tip(&late, "forearm.R")).length();
		assert!(
			early_sep > late_sep + 0.05,
			"mid-sweep is wider than clap contact, {early_sep} vs {late_sep}"
		);
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
