//! Humanoid mapping for [`Beckon`](crate::animations::Beckon).

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Beckon;
use crate::Animation;

impl Animation<HumanoidV0Rig> for Beckon {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let amount = self.gesture_amount(progress);
		if amount < 1e-5 {
			return;
		}

		let mut pose = HumanoidPose::default();
		pose.spine.add_turn(self.torso_turn(progress));

		let neck = self.neck_turn(progress);
		pose.apply_neck_twisted(neck * 0.65, 0.0, 0.0, neck * 0.35, 0.0, 0.0);

		let arm = pose.arm_mut(self.side);
		arm.elbow_flexion += self.elbow_flex(progress);
		arm.aim = Some(ArmAim {
			along: self.humerus_along(progress),
			roll: self.beckon_roll() * self.side.sign(),
		});

		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::Animation;

	const PEAK: f32 = 0.48;

	fn forearm_tip(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		rig.character_point(&format!("forearm.{}", side.suffix()))
	}

	#[test]
	fn beckon_repeatability() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		beckon.apply(&mut a, PEAK);
		beckon.apply(&mut b, PEAK);
		for bone in ["humerus.R", "forearm.R", "upper_back", "lower_neck"] {
			let ra = a.rotation(bone);
			let rb = b.rotation(bone);
			assert!(ra.dot(rb).abs() > 1.0 - 1e-5, "{bone} must match");
		}
		Ok(())
	}

	#[test]
	fn beckon_resample_at_same_progress_is_idempotent() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		let mut rig = HumanoidV0Rig::for_clip_test();
		beckon.apply_for(&mut rig, PEAK);
		let once = rig.pose.clone();
		beckon.apply_for(&mut rig, PEAK);
		assert_eq!(rig.pose, once, "each sample resolves from effective rest");
		Ok(())
	}

	#[test]
	fn beckon_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		beckon.apply_for(&mut entry, 0.0);
		beckon.apply_for(&mut exit, 1.0);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn beckon_loop_boundaries_match_rest() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		let mut at_zero = HumanoidV0Rig::for_clip_test();
		let mut at_one = HumanoidV0Rig::for_clip_test();
		let mut past_one = HumanoidV0Rig::for_clip_test();
		let rest = at_zero.pose.clone();
		beckon.apply_for(&mut at_zero, 0.0);
		beckon.apply_for(&mut at_one, 1.0);
		beckon.apply_for(&mut past_one, 1.25);
		assert_eq!(at_zero.pose, rest);
		assert_eq!(at_one.pose, rest);
		assert_eq!(past_one.pose, rest);
		Ok(())
	}

	#[test]
	fn beckon_interruption_mid_clip_is_stable() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		beckon.apply_for(&mut once, 0.42);
		beckon.apply_for(&mut twice, 0.42);
		beckon.apply_for(&mut twice, 0.42);
		for bone in ["humerus.R", "forearm.R", "upper_back"] {
			let delta = once.rotation(bone).angle_between(twice.rotation(bone));
			assert!(delta < 1e-3, "{bone} drifted {delta}");
		}
		Ok(())
	}

	#[test]
	fn beckon_forearm_reaches_forward_on_gesturing_side() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let bone = format!("forearm.{}", side.suffix());
			let rest = HumanoidV0Rig::for_clip_test();
			let mut posed = HumanoidV0Rig::for_clip_test();
			Beckon::new(side).apply(&mut posed, PEAK);

			let rest_tip = rest.character_point(&bone);
			let posed_tip = forearm_tip(&posed, side);
			assert!(
				posed_tip.z > rest_tip.z + 0.12,
				"{side:?} forearm reaches forward, {posed_tip:?} vs {rest_tip:?}"
			);
			assert!(
				posed_tip.x.signum() == side.sign(),
				"{side:?} forearm stays on its side, {posed_tip:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn beckon_mirrors_forearm_placement() -> anyhow::Result<()> {
		let mut right_rig = HumanoidV0Rig::for_clip_test();
		let mut left_rig = HumanoidV0Rig::for_clip_test();
		Beckon::new(Side::Right).apply(&mut right_rig, PEAK);
		Beckon::new(Side::Left).apply(&mut left_rig, PEAK);

		let r_tip = forearm_tip(&right_rig, Side::Right);
		let l_tip = forearm_tip(&left_rig, Side::Left);
		assert!((r_tip.x + l_tip.x).abs() < 0.05, "mirrored lateral {r_tip:?} {l_tip:?}");
		assert!((r_tip.y - l_tip.y).abs() < 0.06, "matched height {r_tip:?} {l_tip:?}");
		assert!((r_tip.z - l_tip.z).abs() < 0.06, "matched forward reach {r_tip:?} {l_tip:?}");
		Ok(())
	}

	#[test]
	fn beckon_leaves_opposite_arm_at_rest() -> anyhow::Result<()> {
		let beckon = Beckon::new(Side::Right);
		let mut posed = HumanoidV0Rig::for_clip_test();
		beckon.apply(&mut posed, PEAK);

		for bone in ["humerus.L", "forearm.L"] {
			assert!(posed.posed_angle(bone) < 1e-4, "opposite {bone} stays at rest");
		}
		Ok(())
	}

	#[test]
	fn beckon_leaves_legs_at_rest() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		let mut posed = HumanoidV0Rig::for_clip_test();
		beckon.apply(&mut posed, PEAK);

		for bone in ["femur.L", "femur.R", "shin.L", "shin.R", "pelvis.L", "pelvis.R"] {
			assert!(posed.posed_angle(bone) < 0.02, "unwritten {bone} stays at rest");
		}
		Ok(())
	}

	#[test]
	fn beckon_elbow_pulses_during_hold() -> anyhow::Result<()> {
		let beckon = Beckon::default();
		let flex_early = beckon.elbow_flex(0.35);
		let flex_late = beckon.elbow_flex(0.55);
		assert!(
			(flex_early - flex_late).abs() > 0.03,
			"curl should modulate elbow during hold"
		);
		Ok(())
	}

	#[test]
	fn beckon_humerus_aims_forward_in_character_space() -> anyhow::Result<()> {
		let beckon = Beckon::new(Side::Right);
		let mut rig = HumanoidV0Rig::for_clip_test();
		beckon.apply(&mut rig, PEAK);

		let along = beckon.humerus_along(PEAK);
		let humerus_dir = rig.character_length("humerus.R");
		assert!(humerus_dir.dot(along) > 0.8, "aim {along:?} humerus {humerus_dir:?}");
		assert!(humerus_dir.z > 0.35, "humerus reaches forward, {humerus_dir:?}");
		Ok(())
	}
}
