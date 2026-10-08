//! Humanoid mapping for [`Shrug`](crate::animations::Shrug).

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Shrug;
use crate::rigs::humanoid::apply::apply_neck_twisted;
use crate::Animation;

impl Animation<HumanoidV0Rig> for Shrug {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let amount = self.shrug_amount(progress);
		if amount < 1e-5 {
			return;
		}

		let mut pose = HumanoidPose::default();
		let elbow_bend = self.elbow_channel(progress);
		let neck_tilt = self.neck_side_tilt(progress);

		apply_neck_twisted(&mut pose, 0.0, neck_tilt * 0.65, 0.0, 0.0, neck_tilt * 0.35, 0.0);

		let along = self.humerus_along(progress);
		for side in [Side::Left, Side::Right] {
			let arm = pose.arm_mut(side);
			arm.shoulder_lift += self.shoulder_lift(side, progress);
			arm.elbow_flexion += elbow_bend;
			arm.aim = Some(ArmAim { along, roll: 0.0 });
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

	fn peak() -> f32 {
		0.45
	}

	fn forearm_tip(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		rig.character_point(&format!("forearm.{}", side.suffix()))
	}

	#[test]
	fn shrug_repeatability() -> anyhow::Result<()> {
		let shrug = Shrug;
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		shrug.apply(&mut a, peak());
		shrug.apply(&mut b, peak());
		for bone in ["humerus.L", "humerus.R", "forearm.L", "forearm.R", "shoulder.L", "shoulder.R"]
		{
			let ra = a.rotation(bone);
			let rb = b.rotation(bone);
			assert!(ra.dot(rb).abs() > 1.0 - 1e-5, "{bone} must match");
		}
		Ok(())
	}

	#[test]
	fn shrug_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let shrug = Shrug;
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		shrug.apply_for(&mut entry, 0.0);
		shrug.apply_for(&mut exit, 1.0);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn shrug_interruption_mid_clip_is_stable() -> anyhow::Result<()> {
		let shrug = Shrug;
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		shrug.apply_for(&mut once, 0.4);
		shrug.apply_for(&mut twice, 0.4);
		shrug.apply_for(&mut twice, 0.4);
		for bone in ["humerus.L", "humerus.R", "forearm.L", "forearm.R"] {
			let delta = once.rotation(bone).angle_between(twice.rotation(bone));
			assert!(delta < 1e-3, "{bone} drifted {delta}");
		}
		Ok(())
	}

	#[test]
	fn shrug_raises_both_forearm_tips() -> anyhow::Result<()> {
		let shrug = Shrug;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		shrug.apply(&mut posed, peak());

		for side in [Side::Left, Side::Right] {
			let bone = format!("forearm.{}", side.suffix());
			let rest_tip = rest.character_point(&bone);
			let posed_tip = forearm_tip(&posed, side);
			assert!(
				posed_tip.y > rest_tip.y + 0.08,
				"{side:?} forearm rises, {posed_tip:?} vs {rest_tip:?}"
			);
			assert!(
				posed_tip.x.signum() == side.sign(),
				"{side:?} forearm stays on its side of midline, {posed_tip:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn shrug_mirrors_forearm_height() -> anyhow::Result<()> {
		let shrug = Shrug;
		let mut rig = HumanoidV0Rig::for_clip_test();
		shrug.apply(&mut rig, peak());

		let left = forearm_tip(&rig, Side::Left);
		let right = forearm_tip(&rig, Side::Right);
		assert!((left.x + right.x).abs() < 0.06, "mirrored lateral spread {left:?} {right:?}");
		assert!((left.y - right.y).abs() < 0.06, "matched height {left:?} {right:?}");
		Ok(())
	}

	#[test]
	fn shrug_leaves_legs_at_rest() -> anyhow::Result<()> {
		let shrug = Shrug;
		let mut posed = HumanoidV0Rig::for_clip_test();
		shrug.apply(&mut posed, peak());

		for bone in ["femur.L", "femur.R", "shin.L", "shin.R", "pelvis.L", "pelvis.R"] {
			assert!(posed.posed_angle(bone) < 0.02, "unwritten {bone} stays at rest");
		}
		Ok(())
	}

	#[test]
	fn shrug_elbows_flex_at_peak() -> anyhow::Result<()> {
		let shrug = Shrug;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		shrug.apply(&mut posed, peak());

		for side in [Side::Left, Side::Right] {
			let bone = format!("forearm.{}", side.suffix());
			assert!(
				posed.posed_angle(&bone) > rest.posed_angle(&bone) + 0.25,
				"{side:?} elbow flexes at peak"
			);
		}
		Ok(())
	}

	#[test]
	fn shrug_shoulders_lift_at_peak() -> anyhow::Result<()> {
		let shrug = Shrug;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		shrug.apply(&mut posed, peak());

		for side in [Side::Left, Side::Right] {
			let bone = format!("shoulder.{}", side.suffix());
			assert!(
				posed.posed_angle(&bone) > rest.posed_angle(&bone) + 0.15,
				"{side:?} shoulder lifts at peak"
			);
		}
		Ok(())
	}

	fn forearm_tip_dir(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		let s = side.suffix();
		rig.character_point(&format!("forearm.{s}")) - rig.character_point(&format!("humerus.{s}"))
	}

	#[test]
	fn shrug_character_forward_is_positive_z() -> anyhow::Result<()> {
		use character_rigs::authoring::HumanoidPose;
		let mut rig = HumanoidV0Rig::for_clip_test();
		let mut pose = HumanoidPose::default();
		pose.spine.add_root_forward(0.4);
		rig.write_pose(&pose);
		let tipped = rig.rotation("root") * Vec3::Y;
		assert!(tipped.z > 0.2, "for_clip_test fight-forward is +Z, got {tipped:?}");
		Ok(())
	}

	#[test]
	fn shrug_both_humerus_roots_rise_symmetrically() -> anyhow::Result<()> {
		let shrug = Shrug;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		shrug.apply(&mut posed, peak());

		let mut left_dy = 0.0_f32;
		let mut right_dy = 0.0_f32;
		for side in [Side::Left, Side::Right] {
			let s = side.suffix();
			let dy = posed.character_point(&format!("humerus.{s}")).y
				- rest.character_point(&format!("humerus.{s}")).y;
			assert!(dy > 0.08, "{side:?} upper-arm root rises in +Y, dy={dy}");
			match side {
				Side::Left => left_dy = dy,
				Side::Right => right_dy = dy,
			}
		}
		assert!((left_dy - right_dy).abs() < 0.03, "matched lift {left_dy} vs {right_dy}");
		Ok(())
	}

	#[test]
	fn shrug_forearm_tips_point_forward_not_up() -> anyhow::Result<()> {
		let shrug = Shrug;
		let mut posed = HumanoidV0Rig::for_clip_test();
		shrug.apply(&mut posed, peak());

		for side in [Side::Left, Side::Right] {
			let s = side.suffix();
			let dir = forearm_tip_dir(&posed, side);
			let tip = posed.character_point(&format!("forearm.{s}"));
			assert!(
				dir.z.abs() > dir.y.abs(),
				"{side:?} forearm tip is mostly forward, dir={dir:?}"
			);
			assert!(
				tip.x.signum() == side.sign(),
				"{side:?} forearm tip stays on its side of midline, tip={tip:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn shrug_forearm_tip_moves_in_character_space() -> anyhow::Result<()> {
		let shrug = Shrug;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		shrug.apply(&mut posed, peak());

		for side in [Side::Left, Side::Right] {
			let bone = format!("forearm.{}", side.suffix());
			let rest_tip = rest.character_point(&bone);
			let posed_tip = posed.character_point(&bone);
			assert!(
				(posed_tip - rest_tip).length() > 0.12,
				"{side:?} forearm tip moves, {posed_tip:?} vs {rest_tip:?}"
			);
		}
		Ok(())
	}
}
