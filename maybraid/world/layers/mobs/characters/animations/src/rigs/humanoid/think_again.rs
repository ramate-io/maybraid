//! Humanoid mapping for [`ThinkAgain`](crate::animations::ThinkAgain).

use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::ThinkAgain;
use crate::Animation;

impl Animation<HumanoidV0Rig> for ThinkAgain {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		if self.gesture_amount(progress) < 1e-5 {
			return;
		}

		let mut pose = HumanoidPose::default();
		let arm = pose.arm_mut(self.side);
		arm.elbow_flexion += self.elbow_flexion(progress);
		arm.aim =
			Some(ArmAim { along: self.humerus_along(progress), roll: self.humerus_roll(progress) });

		rig.write_pose(&pose);
	}
}

fn segment_tip(rig: &HumanoidV0Rig, bone: &str) -> Vec3 {
	let Some(id) = rig.binding.definition.id(bone) else {
		panic!("missing bone {bone}");
	};
	let len = rig.binding.effective_rest.local[id.index()].translation.length();
	rig.character_point(bone) + rig.character_length(bone) * len
}

fn forearm_from_vertical(rig: &HumanoidV0Rig, side: Side) -> f32 {
	let bone = format!("forearm.{}", side.suffix());
	let dir = rig.character_length(&bone);
	dir.x.atan2(dir.y)
}

fn palm_inward_dot(rig: &HumanoidV0Rig, side: Side) -> f32 {
	let bone = format!("forearm.{}", side.suffix());
	let local_palm = match side {
		Side::Right => Vec3::X,
		Side::Left => Vec3::NEG_X,
	};
	let palm = rig.rotation(&bone) * local_palm;
	let inward = Vec3::new(side.sign(), 0.0, 0.0);
	palm.dot(inward)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Animation;

	fn think_hold() -> f32 {
		0.38
	}

	fn again_hold() -> f32 {
		0.815
	}

	#[test]
	fn think_again_repeatability() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut a, think_hold());
		clip.apply(&mut b, think_hold());
		for bone in ["humerus.R", "forearm.R"] {
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
	fn think_again_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		clip.apply_for(&mut entry, 0.0);
		clip.apply_for(&mut exit, 1.0);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn think_again_interruption_mid_clip_is_stable() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		clip.apply_for(&mut once, 0.35);
		clip.apply_for(&mut twice, 0.35);
		clip.apply_for(&mut twice, 0.35);
		for bone in ["humerus.R", "forearm.R"] {
			let delta = once.rotation(bone).angle_between(twice.rotation(bone));
			assert!(delta < 1e-3, "{bone} drifted {delta}");
		}
		Ok(())
	}

	#[test]
	fn think_again_humerus_stays_horizontal_lateral() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut rig, think_hold());

		let humerus = rig.character_length("humerus.R");
		let elevation = humerus.y.atan2(humerus.x.abs()).to_degrees();
		assert!(elevation.abs() < 10.0, "humerus elevation {elevation:.1}°, dir {humerus:?}");
		assert!(humerus.x > 0.85, "lateral +X, got {humerus:?}");
		Ok(())
	}

	#[test]
	fn think_again_humerus_barely_moves_between_key_poses() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut think = HumanoidV0Rig::for_clip_test();
		let mut again = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut think, think_hold());
		clip.apply(&mut again, again_hold());

		let think_dir = think.character_length("humerus.R");
		let again_dir = again.character_length("humerus.R");
		let delta = think_dir.angle_between(again_dir).to_degrees();
		assert!(delta < 3.0, "humerus drift {delta:.1}° {think_dir:?} vs {again_dir:?}");
		Ok(())
	}

	#[test]
	fn think_again_elbow_sweeps_about_sixty_degrees() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut think = HumanoidV0Rig::for_clip_test();
		let mut again = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut think, think_hold());
		clip.apply(&mut again, again_hold());

		let think_angle = think.posed_angle("forearm.R");
		let again_angle = again.posed_angle("forearm.R");
		let delta = (think_angle - again_angle).abs();
		assert!(delta > 0.45 && delta < 1.15, "elbow sweep {delta:.2} rad");
		Ok(())
	}

	#[test]
	fn think_again_hand_near_head_at_think() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut rig, think_hold());

		let tip = segment_tip(&rig, "forearm.R");
		let elbow = rig.character_point("forearm.R");
		let head_y = rig.character_point("upper_neck").y + 0.12;
		assert!(tip.y > head_y - 0.08, "hand near head height, tip {tip:?}");
		assert!(tip.x < elbow.x + 0.05, "hand inboard of elbow, tip {tip:?} elbow {elbow:?}");
		Ok(())
	}

	#[test]
	fn think_again_hand_outboard_at_again() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut think = HumanoidV0Rig::for_clip_test();
		let mut again = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut think, think_hold());
		clip.apply(&mut again, again_hold());

		let think_tip = segment_tip(&think, "forearm.R");
		let again_tip = segment_tip(&again, "forearm.R");
		assert!(
			again_tip.x > think_tip.x + 0.08,
			"Again opens outboard, think {think_tip:?} again {again_tip:?}"
		);
		let head_x = rig_head_half_width(&again);
		assert!(again_tip.x < head_x + 0.25, "hand stays modestly outside head, tip {again_tip:?}");
		Ok(())
	}

	#[test]
	fn think_again_hold_pose_is_stable() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut early = HumanoidV0Rig::for_clip_test();
		let mut late = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut early, 0.22);
		clip.apply(&mut late, 0.52);
		let early_tip = segment_tip(&early, "forearm.R");
		let late_tip = segment_tip(&late, "forearm.R");
		assert!((early_tip - late_tip).length() < 0.04, "hold drift {early_tip:?} vs {late_tip:?}");
		Ok(())
	}

	#[test]
	fn think_again_palm_faces_inward() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut rig, think_hold());
		assert!(palm_inward_dot(&rig, Side::Right) > 0.35, "palm should face inward");
		Ok(())
	}

	#[test]
	fn think_again_mirrors_for_left_side() -> anyhow::Result<()> {
		let right = ThinkAgain::default().with_side(Side::Right);
		let left = ThinkAgain::default().with_side(Side::Left);
		let mut r = HumanoidV0Rig::for_clip_test();
		let mut l = HumanoidV0Rig::for_clip_test();
		right.apply(&mut r, think_hold());
		left.apply(&mut l, think_hold());

		let r_tip = segment_tip(&r, "forearm.R");
		let l_tip = segment_tip(&l, "forearm.L");
		assert!((r_tip.x + l_tip.x).abs() < 0.06, "mirrored tips {r_tip:?} {l_tip:?}");
		assert!((r_tip.y - l_tip.y).abs() < 0.08, "matched height {r_tip:?} {l_tip:?}");
		assert!(palm_inward_dot(&l, Side::Left) > 0.35, "left palm inward");
		Ok(())
	}

	#[test]
	fn think_again_leaves_opposite_arm_at_rest() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut posed = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut posed, think_hold());

		for bone in ["humerus.L", "forearm.L"] {
			assert!(posed.posed_angle(bone) < 1e-4, "opposite {bone} stays at rest");
		}
		Ok(())
	}

	#[test]
	fn think_again_tip_stays_on_gesture_side() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let bone = format!("forearm.{}", side.suffix());
			let rest = HumanoidV0Rig::for_clip_test();
			let mut posed = HumanoidV0Rig::for_clip_test();
			ThinkAgain::default().with_side(side).apply(&mut posed, think_hold());
			let rest_tip = segment_tip(&rest, &bone);
			let posed_tip = segment_tip(&posed, &bone);
			assert!(
				posed_tip.x.signum() == rest_tip.x.signum(),
				"{side:?} must not cross midline, {posed_tip:?} vs {rest_tip:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn think_again_samples_are_continuous() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut prev = segment_tip(&HumanoidV0Rig::for_clip_test(), "forearm.R");
		let mut rig = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut rig, 0.0);
		prev = segment_tip(&rig, "forearm.R");
		for i in 1..=64 {
			let t = i as f32 / 64.0;
			let mut rig = HumanoidV0Rig::for_clip_test();
			clip.apply(&mut rig, t);
			let tip = segment_tip(&rig, "forearm.R");
			if clip.gesture_amount(t) > 0.05 && clip.gesture_amount((i - 1) as f32 / 64.0) > 0.05 {
				let jump = (tip - prev).length();
				assert!(jump < 0.18, "discontinuity at {t:.3}: {jump:.3} {prev:?} -> {tip:?}");
			}
			prev = tip;
		}
		Ok(())
	}

	#[test]
	fn think_again_forearm_angles_match_spec() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut think = HumanoidV0Rig::for_clip_test();
		let mut again = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut think, think_hold());
		clip.apply(&mut again, again_hold());

		let think_deg = forearm_from_vertical(&think, Side::Right).to_degrees();
		let again_deg = forearm_from_vertical(&again, Side::Right).to_degrees();
		assert!(think_deg < -35.0 && think_deg > -55.0, "Think tilt {think_deg:.1}°");
		assert!(again_deg > 5.0 && again_deg < 25.0, "Again tilt {again_deg:.1}°");
		Ok(())
	}

	fn rig_head_half_width(rig: &HumanoidV0Rig) -> f32 {
		rig.character_point("upper_neck").x.abs() + 0.08
	}
}
