//! Humanoid mapping for [`HeadNod`](crate::animations::HeadNod).

use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use crate::animations::HeadNod;
use crate::Animation;

impl Animation<HumanoidV0Rig> for HeadNod {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let nod = self.nod_twist(progress);
		let spine = self.spine_follow(progress);
		if nod < 1e-5 && spine < 1e-5 {
			return;
		}

		let mut pose = HumanoidPose::default();
		pose.apply_spine_pitch(spine);
		pose.apply_neck_twisted(0.0, 0.0, nod * 0.35, 0.0, 0.0, nod * 0.65);

		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;

	use crate::animations::HeadNod;
	use crate::Animation;

	fn first_peak() -> f32 {
		0.24
	}

	fn second_peak() -> f32 {
		0.54
	}

	fn head_tip(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("upper_neck")
	}

	#[test]
	fn head_nod_repeatability() -> anyhow::Result<()> {
		let nod = HeadNod;
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		nod.apply(&mut a, first_peak());
		nod.apply(&mut b, first_peak());
		for bone in ["lower_neck", "upper_neck", "upper_back"] {
			let ra = a.rotation(bone);
			let rb = b.rotation(bone);
			assert!(ra.dot(rb).abs() > 1.0 - 1e-5, "{bone} must match");
		}
		Ok(())
	}

	#[test]
	fn head_nod_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let nod = HeadNod;
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		nod.apply_for(&mut entry, 0.0);
		nod.apply_for(&mut exit, 1.0);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn head_nod_interruption_mid_clip_is_stable() -> anyhow::Result<()> {
		let nod = HeadNod;
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		nod.apply_for(&mut once, 0.38);
		nod.apply_for(&mut twice, 0.38);
		nod.apply_for(&mut twice, 0.38);
		for bone in ["lower_neck", "upper_neck", "upper_back"] {
			let delta = once.rotation(bone).angle_between(twice.rotation(bone));
			assert!(delta < 1e-3, "{bone} drifted {delta}");
		}
		Ok(())
	}

	#[test]
	fn head_nod_loop_boundary_matches_rest() -> anyhow::Result<()> {
		let nod = HeadNod;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut at_zero = HumanoidV0Rig::for_clip_test();
		let mut at_one = HumanoidV0Rig::for_clip_test();
		nod.apply(&mut at_zero, 0.0);
		nod.apply(&mut at_one, 1.0);
		let rest_head = head_tip(&rest);
		assert!((head_tip(&at_zero) - rest_head).length() < 1e-3);
		assert!((head_tip(&at_one) - rest_head).length() < 1e-3);
		Ok(())
	}

	#[test]
	fn head_nod_dips_head_forward_at_each_peak() -> anyhow::Result<()> {
		let nod = HeadNod;
		let rest = HumanoidV0Rig::for_clip_test();
		let rest_head = head_tip(&rest);
		for peak in [first_peak(), second_peak()] {
			let mut posed = HumanoidV0Rig::for_clip_test();
			nod.apply(&mut posed, peak);
			let posed_head = head_tip(&posed);
			assert!(
				posed_head.z > rest_head.z + 0.02,
				"chin dip moves head forward at {peak}, {posed_head:?} vs {rest_head:?}"
			);
			assert!(
				posed_head.y < rest_head.y + 0.01,
				"nod should not lift the head, {posed_head:?} vs {rest_head:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn head_nod_is_bilateral_at_peak() -> anyhow::Result<()> {
		let nod = HeadNod;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		nod.apply(&mut posed, first_peak());

		for bone in ["humerus.L", "humerus.R", "forearm.L", "forearm.R"] {
			assert!(posed.posed_angle(bone) < 0.02, "{bone} stays unauthored");
		}

		let lower = posed.rotation("lower_neck");
		let upper = posed.rotation("upper_neck");
		let rest_lower = rest.rotation("lower_neck");
		let rest_upper = rest.rotation("upper_neck");
		assert!(
			lower.angle_between(rest_lower) > 0.02,
			"lower neck participates in the nod"
		);
		assert!(
			upper.angle_between(rest_upper) > 0.02,
			"upper neck participates in the nod"
		);
		Ok(())
	}

	#[test]
	fn head_nod_unwritten_bones_stay_at_rest() -> anyhow::Result<()> {
		let nod = HeadNod;
		let mut posed = HumanoidV0Rig::for_clip_test();
		nod.apply(&mut posed, first_peak());

		for bone in ["humerus.L", "humerus.R", "forearm.L", "forearm.R"] {
			assert!(posed.posed_angle(bone) < 1e-4, "arm {bone} stays at rest");
		}
		for bone in ["femur.L", "femur.R", "shin.L", "shin.R"] {
			let angle = posed.posed_angle(bone);
			assert!(angle < 0.02, "leg {bone} stays at rest (angle={angle})");
		}
		Ok(())
	}
}
