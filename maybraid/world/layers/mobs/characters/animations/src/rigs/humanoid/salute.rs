//! Humanoid mapping for [`Salute`](crate::animations::Salute).

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Salute;
use crate::rigs::humanoid::apply::apply_neck_twisted;
use crate::Animation;

impl Animation<HumanoidV0Rig> for Salute {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let amount = self.salute_amount(progress);
		if amount < 1e-5 {
			return;
		}

		let mut pose = HumanoidPose::default();
		let salute_side = self.side;

		apply_neck_twisted(
			&mut pose,
			self.neck_turn(progress) * 0.65,
			0.0,
			self.neck_nod(progress) * 0.35,
			self.neck_turn(progress) * 0.35,
			0.0,
			self.neck_nod(progress) * 0.65,
		);

		let arm = pose.arm_mut(salute_side);
		arm.elbow_flexion += self.salute_elbow(progress);
		arm.aim = Some(ArmAim {
			along: self.humerus_along(progress),
			roll: self.salute_roll(progress) * salute_side.sign(),
		});

		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::Animation;

	fn peak() -> f32 {
		0.45
	}

	#[test]
	fn salute_repeatability() -> anyhow::Result<()> {
		let salute = Salute::default();
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		salute.apply(&mut a, peak());
		salute.apply(&mut b, peak());
		for bone in ["humerus.R", "forearm.R", "lower_neck", "upper_neck"] {
			let ra = a.rotation(bone);
			let rb = b.rotation(bone);
			assert!((ra.x - rb.x).abs() < 1e-5, "{bone} x");
			assert!((ra.y - rb.y).abs() < 1e-5, "{bone} y");
			assert!((ra.z - rb.z).abs() < 1e-5, "{bone} z");
			assert!((ra.w - rb.w).abs() < 1e-5, "{bone} w");
		}
		Ok(())
	}

	#[test]
	fn salute_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let salute = Salute::default();
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		salute.apply_for(&mut entry, 0.0);
		salute.apply_for(&mut exit, 1.0);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn salute_interruption_mid_clip_is_stable() -> anyhow::Result<()> {
		let salute = Salute::default();
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		salute.apply_for(&mut once, 0.35);
		salute.apply_for(&mut twice, 0.35);
		salute.apply_for(&mut twice, 0.35);
		for bone in ["humerus.R", "forearm.R", "lower_neck", "upper_neck"] {
			let delta = once.rotation(bone).angle_between(twice.rotation(bone));
			assert!(delta < 1e-3, "{bone} drifted {delta}");
		}
		Ok(())
	}

	#[test]
	fn salute_raises_forearm_tip_at_peak() -> anyhow::Result<()> {
		let salute = Salute::default().with_side(Side::Right);
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		salute.apply(&mut posed, peak());

		let rest_tip = rest.character_point("forearm.R");
		let posed_tip = posed.character_point("forearm.R");
		assert!(
			posed_tip.y > rest_tip.y + 0.15,
			"forearm tip rises to brow, {posed_tip:?} vs {rest_tip:?}"
		);
		assert!(
			posed_tip.x > rest_tip.x + 0.05,
			"right salute moves inboard (+X), {posed_tip:?} vs {rest_tip:?}"
		);
		Ok(())
	}

	#[test]
	fn salute_mirrors_forearm_inboard_for_left_side() -> anyhow::Result<()> {
		let right = Salute::default().with_side(Side::Right);
		let left = Salute::default().with_side(Side::Left);
		let mut r = HumanoidV0Rig::for_clip_test();
		let mut l = HumanoidV0Rig::for_clip_test();
		right.apply(&mut r, peak());
		left.apply(&mut l, peak());

		let r_tip = r.character_point("forearm.R");
		let l_tip = l.character_point("forearm.L");
		assert!(r_tip.x < 0.0 && l_tip.x > 0.0, "brow tips cross the sagittal plane: {r_tip:?} {l_tip:?}");
		assert!((r_tip.x + l_tip.x).abs() < 0.02, "mirrored brow placement {r_tip:?} {l_tip:?}");
		assert!((r_tip.y - l_tip.y).abs() < 0.08, "matched height {r_tip:?} {l_tip:?}");
		Ok(())
	}

	#[test]
	fn salute_leaves_opposite_arm_at_rest() -> anyhow::Result<()> {
		let salute = Salute::default().with_side(Side::Right);
		let mut posed = HumanoidV0Rig::for_clip_test();
		salute.apply(&mut posed, peak());

		for bone in ["humerus.L", "forearm.L"] {
			assert!(posed.posed_angle(bone) < 1e-4, "opposite {bone} stays at rest");
		}
		for bone in ["femur.L", "femur.R", "shin.L", "shin.R"] {
			let angle = posed.posed_angle(bone);
			assert!(angle < 0.02, "unwritten leg {bone} stays at rest (angle={angle})");
		}
		Ok(())
	}

	#[test]
	fn salute_elbow_flexes_at_peak() -> anyhow::Result<()> {
		let salute = Salute::default().with_side(Side::Right);
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		salute.apply(&mut posed, peak());

		assert!(
			posed.posed_angle("forearm.R") > rest.posed_angle("forearm.R") + 0.8,
			"salute closes the elbow"
		);
		Ok(())
	}

	#[test]
	fn salute_humerus_aims_up_in_character_space() -> anyhow::Result<()> {
		let salute = Salute::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::for_clip_test();
		salute.apply(&mut rig, peak());

		let along = salute.humerus_along(peak());
		let humerus_dir = rig.character_length("humerus.R");
		assert!(humerus_dir.dot(along) > 0.85, "aim {along:?} humerus {humerus_dir:?}");
		assert!(humerus_dir.y > 0.5, "humerus points up, {humerus_dir:?}");
		Ok(())
	}
}
