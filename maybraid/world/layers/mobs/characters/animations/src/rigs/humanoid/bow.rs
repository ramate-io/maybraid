//! Humanoid mapping for [`Bow`](crate::animations::Bow).

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Bow;
use crate::Animation;

impl Animation<HumanoidV0Rig> for Bow {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let amount = self.bow_amount(progress);
		if amount < 1e-5 {
			return;
		}

		let mut pose = HumanoidPose::default();
		pose.apply_spine_pitch(self.spine_pitch(progress));
		pose.apply_root(self.root_lean(progress));

		let femur = self.femur_swing(progress);
		let shin = self.shin_flex(progress);
		let hip = self.hip_fold(progress);
		pose.apply_leg(Side::Left, femur, shin);
		pose.apply_leg(Side::Right, femur, shin);
		pose.apply_hip_fold(Side::Left, hip);
		pose.apply_hip_fold(Side::Right, hip);

		let nod = self.neck_nod(progress);
		pose.apply_neck_twisted(0.0, 0.0, nod * 0.65, 0.0, 0.0, nod * 0.35);

		let along = self.humerus_along(progress);
		let elbow = self.elbow_flex(progress);
		for side in [Side::Left, Side::Right] {
			let arm = pose.arm_mut(side);
			arm.elbow_flexion += elbow;
			arm.aim = Some(ArmAim { along, roll: 0.0 });
		}

		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;

	use super::*;
	use crate::Animation;

	fn peak() -> f32 {
		0.43
	}

	fn head_point(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("upper_neck")
	}

	#[test]
	fn bow_repeatability() -> anyhow::Result<()> {
		let bow = Bow;
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		bow.apply(&mut a, peak());
		bow.apply(&mut b, peak());
		for bone in ["upper_back", "root", "femur.L", "femur.R", "forearm.L", "forearm.R"] {
			let ra = a.rotation(bone);
			let rb = b.rotation(bone);
			assert!(ra.dot(rb).abs() > 1.0 - 1e-5, "{bone} must match");
		}
		Ok(())
	}

	#[test]
	fn bow_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let bow = Bow;
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		bow.apply_for(&mut entry, 0.0);
		bow.apply_for(&mut exit, 1.0);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn bow_interruption_mid_clip_is_stable() -> anyhow::Result<()> {
		let bow = Bow;
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		bow.apply_for(&mut once, 0.38);
		bow.apply_for(&mut twice, 0.38);
		bow.apply_for(&mut twice, 0.38);
		for bone in ["upper_back", "root", "forearm.L", "forearm.R"] {
			let delta = once.rotation(bone).angle_between(twice.rotation(bone));
			assert!(delta < 1e-3, "{bone} drifted {delta}");
		}
		Ok(())
	}

	#[test]
	fn bow_lowers_head_at_peak() -> anyhow::Result<()> {
		let bow = Bow;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		bow.apply(&mut posed, peak());

		let rest_head = head_point(&rest);
		let posed_head = head_point(&posed);
		assert!(
			posed_head.y < rest_head.y - 0.08,
			"head dips with torso, {posed_head:?} vs {rest_head:?}"
		);
		assert!(
			posed_head.z > rest_head.z - 0.05,
			"head should not lurch backward, {posed_head:?} vs {rest_head:?}"
		);
		Ok(())
	}

	#[test]
	fn bow_mirrors_arm_pose() -> anyhow::Result<()> {
		let bow = Bow;
		let mut rig = HumanoidV0Rig::for_clip_test();
		bow.apply(&mut rig, peak());

		for side in [Side::Left, Side::Right] {
			let bone = format!("forearm.{}", side.suffix());
			let tip = rig.character_point(&bone);
			assert!(tip.x.signum() == side.sign(), "{side:?} arm stays on its side, {tip:?}");
		}

		let left = rig.character_point("forearm.L");
		let right = rig.character_point("forearm.R");
		assert!((left.y - right.y).abs() < 0.05, "matched forearm height {left:?} {right:?}");
		assert!((left.x + right.x).abs() < 0.08, "mirrored spread {left:?} {right:?}");
		Ok(())
	}

	#[test]
	fn bow_pelvis_flex_is_bilateral_at_peak() -> anyhow::Result<()> {
		let bow = Bow;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		bow.apply(&mut posed, peak());

		let left = posed.posed_angle("pelvis.L");
		let right = posed.posed_angle("pelvis.R");
		assert!(left > rest.posed_angle("pelvis.L") + 0.04, "hip fold reads on left");
		assert!((left - right).abs() < 0.03, "mirrored pelvis flex {left} {right}");
		Ok(())
	}

	#[test]
	fn bow_loop_boundary_matches_rest() -> anyhow::Result<()> {
		let bow = Bow;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut at_one = HumanoidV0Rig::for_clip_test();
		let mut at_zero = HumanoidV0Rig::for_clip_test();
		bow.apply(&mut at_zero, 0.0);
		bow.apply(&mut at_one, 1.0);
		let rest_head = head_point(&rest);
		let zero_head = head_point(&at_zero);
		let one_head = head_point(&at_one);
		assert!((zero_head - rest_head).length() < 1e-3);
		assert!((one_head - rest_head).length() < 1e-3);
		Ok(())
	}
}
