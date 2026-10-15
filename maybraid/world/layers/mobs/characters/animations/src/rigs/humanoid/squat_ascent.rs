use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::SquatAscent;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for SquatAscent {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let femur_swing = self.femur_swing(progress);
		let shin_flex = self.shin_flex(progress);

		pose.apply_leg(Side::Left, femur_swing, shin_flex);
		pose.apply_leg(Side::Right, femur_swing, shin_flex);
		let hip = self.hip_fold(progress);
		if hip.abs() > f32::EPSILON {
			pose.apply_hip_fold(Side::Left, hip);
			pose.apply_hip_fold(Side::Right, hip);
		}
		pose.apply_spine_pitch(self.root_swing(progress));
		for side in [Side::Left, Side::Right] {
			let flex = self.shoulder_flex(side, progress);
			if flex.abs() > f32::EPSILON {
				pose.apply_arm(side, 0.0, flex, 0.0, 0.0, 0.0);
			}
		}
		rig.write_pose(&pose);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;
	use crate::animations::Squat;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn shin_end(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		let name = format!("shin.{}", side.suffix());
		rig.character_point(&name) + rig.character_length(&name)
	}

	fn max_tip_jump(ascent_p: f32, descent_p: f32) -> f32 {
		use crate::animations::SquatDescent;

		let mut ascent_rig = HumanoidV0Rig::for_clip_test();
		let mut descent_rig = HumanoidV0Rig::for_clip_test();
		SquatAscent::default().apply(&mut ascent_rig, ascent_p);
		SquatDescent::default().apply(&mut descent_rig, descent_p);
		let leg_bones = ["femur.L", "femur.R", "shin.L", "shin.R", "root", "pelvis.L", "pelvis.R"];
		leg_bones
			.iter()
			.map(|name| {
				let a = ascent_rig.character_point(name) + ascent_rig.character_length(name);
				let b = descent_rig.character_point(name) + descent_rig.character_length(name);
				(a - b).length()
			})
			.fold(0.0_f32, f32::max)
	}

	#[test]
	fn start_matches_held_squat() -> anyhow::Result<()> {
		let mut ascent_rig = HumanoidV0Rig::for_clip_test();
		let mut held_rig = HumanoidV0Rig::for_clip_test();
		SquatAscent::default().apply(&mut ascent_rig, 0.0);
		Squat::held().apply(&mut held_rig, 1.0);

		for name in ascent_rig.animation_bone_names() {
			let a = ascent_rig.rotation(name);
			let b = held_rig.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "rotation mismatch on {name}");
		}
		Ok(())
	}

	#[test]
	fn end_matches_upright() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatAscent::default().apply(&mut rig, 1.0);
		let rest = HumanoidV0Rig::for_clip_test();
		for name in ["femur.L", "shin.L", "root"] {
			let posed = tip(&rig, name);
			let rest_tip = tip(&rest, name);
			assert!((posed - rest_tip).length() < 1e-3, "{name} should stay at rest");
		}
		Ok(())
	}

	#[test]
	fn both_femurs_unfold_sagittally_at_mid_ascent() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatAscent::default().apply(&mut rig, 0.5);
		let held = Squat::held().femur_peak.abs();
		let angle = rig.posed_angle("femur.L");
		assert!(angle > 0.05 && angle < held, "mid ascent should sit between upright and held");
		assert!(
			(rig.posed_angle("femur.L") - rig.posed_angle("femur.R")).abs() < 1e-4,
			"both legs share flexion"
		);
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		let ascent = SquatAscent::default();
		ascent.apply(&mut once, 0.4);
		ascent.apply(&mut twice, 0.4);
		ascent.apply(&mut twice, 0.4);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn transition_from_held_squat_blends_into_mid_ascent() -> anyhow::Result<()> {
		use crate::animations::{Squat, Transition};
		use character_rigs::authoring::ArmatureOffset;

		let mut rig = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut rig, 1.0);
		let from_pose = rig.pose.clone();
		let transition =
			Transition::from_visible(SquatAscent::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.5, 0.5);
		assert!(
			rig.posed_angle("femur.L") > 0.05
				&& rig.posed_angle("femur.L") < Squat::held().femur_peak.abs(),
			"blend should land between held squat and upright"
		);
		Ok(())
	}

	#[test]
	fn ascent_endpoints_match_descent_inverse() -> anyhow::Result<()> {
		use crate::animations::SquatDescent;

		let mut ascent = HumanoidV0Rig::for_clip_test();
		let mut descent = HumanoidV0Rig::for_clip_test();
		SquatAscent::default().apply(&mut ascent, 0.35);
		SquatDescent::default().apply(&mut descent, 1.0 - 0.35);
		for name in ["femur.L", "shin.L", "root", "pelvis.L"] {
			let a = ascent.rotation(name);
			let b = descent.rotation(name);
			assert!(
				a.dot(b).abs() > 1.0 - 1e-4,
				"ascent depth should match inverted descent on {name}"
			);
		}
		Ok(())
	}

	#[test]
	fn body_proportions_preserve_rotation_endpoints() -> anyhow::Result<()> {
		use character_rigs::humanoid::LegSegmentLengths;

		let mut default_rig = HumanoidV0Rig::for_clip_test();
		let mut long_rig = HumanoidV0Rig::for_clip_test();
		long_rig.segment_lengths = LegSegmentLengths { femur: 0.625, shin: 0.625 };
		let ascent = SquatAscent::default();
		for progress in [0.0, 1.0] {
			ascent.apply(&mut default_rig, progress);
			ascent.apply(&mut long_rig, progress);
			for name in ["femur.L", "femur.R", "shin.L", "shin.R"] {
				let a = default_rig.rotation(name);
				let b = long_rig.rotation(name);
				assert!(
					a.dot(b).abs() > 1.0 - 1e-5,
					"{name} at {progress} should not depend on segment length"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn shoulder_lead_raises_both_sides_symmetrically() -> anyhow::Result<()> {
		use character_rigs::authoring::HumanoidPose;

		let rest = HumanoidV0Rig::for_clip_test();
		let peak_p = 0.15;
		let lead = SquatAscent::default().shoulder_lead(peak_p);

		// Same shoulder_forward on both sides (pre-fix) skews character-space height.
		let mut old_rig = HumanoidV0Rig::for_clip_test();
		let mut old_pose = HumanoidPose::default();
		old_pose.apply_arm(Side::Left, lead, 0.0, 0.0, 0.0, 0.0);
		old_pose.apply_arm(Side::Right, lead, 0.0, 0.0, 0.0, 0.0);
		old_rig.write_pose(&old_pose);
		let old_left_y = old_rig.character_point("shoulder.L").y;
		let old_right_y = old_rig.character_point("shoulder.R").y;

		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatAscent::default().apply(&mut rig, peak_p);
		let left = rig.character_point("shoulder.L");
		let right = rig.character_point("shoulder.R");
		let humerus_l = rig.character_point("humerus.L");
		let humerus_r = rig.character_point("humerus.R");
		let old_skew = (old_left_y - old_right_y).abs();
		let new_skew = (left.y - right.y).abs();
		assert!(
			new_skew < 0.02 && new_skew <= old_skew + 1e-4,
			"shoulders stay level with flex+sign (old skew {old_skew:.4}, new {new_skew:.4}) L={left:?} R={right:?}"
		);
		let rest_left = rest.character_point("shoulder.L");
		let rest_right = rest.character_point("shoulder.R");
		let lift_l = left.y - rest_left.y;
		let lift_r = right.y - rest_right.y;
		assert!(
			lift_l.signum() == lift_r.signum() && lift_l.abs() > 0.005,
			"both shoulders should move together in Y at peak (ΔL={lift_l:.4} ΔR={lift_r:.4})"
		);
		assert!(
			(humerus_l.y - rest.character_point("humerus.L").y).signum()
				== (humerus_r.y - rest.character_point("humerus.R").y).signum(),
			"humerus roots should follow shoulders, L={humerus_l:?} R={humerus_r:?}"
		);
		let mut held = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut held, 1.0);
		let mut upright = HumanoidV0Rig::for_clip_test();
		SquatAscent::default().apply(&mut upright, 1.0);
		for (progress, reference) in [(0.0, &held), (1.0, &upright)] {
			let mut endpoint = HumanoidV0Rig::for_clip_test();
			SquatAscent::default().apply(&mut endpoint, progress);
			for side in [Side::Left, Side::Right] {
				let shoulder = format!("shoulder.{}", side.suffix());
				assert!(
					(endpoint.character_point(&shoulder) - reference.character_point(&shoulder))
						.length() < 1e-3,
					"shoulder lead off at progress {progress} on {shoulder}"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn ascent_descent_toggle_stays_continuous_at_matched_depth() -> anyhow::Result<()> {
		use crate::animations::SquatDescent;

		const MAX_LEG_JUMP_M: f32 = 0.02;
		const MAX_ARM_JUMP_M: f32 = 0.02;
		for blend in [0.25, 0.5, 0.75] {
			let leg_jump = max_tip_jump(1.0 - blend, blend);
			assert!(
				leg_jump < MAX_LEG_JUMP_M,
				"toggle at blend {blend}: leg tip jump {leg_jump:.4} m exceeds {MAX_LEG_JUMP_M} m"
			);

			let mut ascent_rig = HumanoidV0Rig::for_clip_test();
			let mut descent_rig = HumanoidV0Rig::for_clip_test();
			SquatAscent::default().apply(&mut ascent_rig, 1.0 - blend);
			SquatDescent::default().apply(&mut descent_rig, blend);
			let arm_jump = ["shoulder.L", "shoulder.R", "humerus.L", "humerus.R"]
				.iter()
				.map(|name| {
					let a = ascent_rig.character_point(name) + ascent_rig.character_length(name);
					let b = descent_rig.character_point(name) + descent_rig.character_length(name);
					(a - b).length()
				})
				.fold(0.0_f32, f32::max);
			assert!(
				arm_jump < MAX_ARM_JUMP_M,
				"toggle at blend {blend}: arm tip jump {arm_jump:.4} m exceeds {MAX_ARM_JUMP_M} m \
				 (shoulder lead is zero at matched depth)"
			);
		}
		Ok(())
	}

	#[test]
	fn feet_match_descent_and_held_squat_endpoints() -> anyhow::Result<()> {
		use crate::animations::{Squat, SquatDescent};

		const MAX_FOOT_DRIFT_M: f32 = 0.02;
		let rest = HumanoidV0Rig::for_clip_test();
		let mut held = HumanoidV0Rig::for_clip_test();
		Squat::held().apply(&mut held, 1.0);
		for blend in [0.0, 0.25, 0.5, 0.75, 1.0] {
			let mut ascent = HumanoidV0Rig::for_clip_test();
			SquatAscent::default().apply(&mut ascent, 1.0 - blend);
			let mut descent = HumanoidV0Rig::for_clip_test();
			SquatDescent::default().apply(&mut descent, blend);
			for side in [Side::Left, Side::Right] {
				let ascent_foot = shin_end(&ascent, side);
				let descent_foot = shin_end(&descent, side);
				assert!(
					(ascent_foot - descent_foot).length() < MAX_FOOT_DRIFT_M,
					"matched depth blend={blend}: {:?} vs {:?}",
					ascent_foot,
					descent_foot
				);
			}
		}
		for side in [Side::Left, Side::Right] {
			let mut start = HumanoidV0Rig::for_clip_test();
			SquatAscent::default().apply(&mut start, 0.0);
			let held_foot = shin_end(&held, side);
			let start_foot = shin_end(&start, side);
			assert!(
				(start_foot - held_foot).length() < MAX_FOOT_DRIFT_M,
				"ascent start should match held squat foot on {side:?}"
			);
			let rest_foot = shin_end(&rest, side);
			let mut end = HumanoidV0Rig::for_clip_test();
			SquatAscent::default().apply(&mut end, 1.0);
			let end_foot = shin_end(&end, side);
			assert!(
				(end_foot - rest_foot).length() < MAX_FOOT_DRIFT_M,
				"ascent end should match upright foot on {side:?}"
			);
		}
		let effects = SquatAscent::default().effects_for(&rest, 0.5);
		assert!(effects.is_identity(), "ascent armature offset must stay visual-only / identity");
		Ok(())
	}

	#[test]
	fn jump_interrupt_from_visible_mid_ascent() -> anyhow::Result<()> {
		use crate::animations::{JumpSegment, Transition, TwoFootedJump};
		use character_rigs::authoring::ArmatureOffset;

		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatAscent::default().apply(&mut rig, 0.4);
		let from_pose = rig.pose.clone();
		let jump = TwoFootedJump::default();
		let lengths = rig.segment_lengths;
		let windup = jump.timings(lengths).squat_descent_duration * 0.5;
		let transition =
			Transition::from_visible(jump.clone(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, windup, 0.0);
		assert_eq!(jump.segment(lengths, windup).0, JumpSegment::Squat);
		assert!(
			rig.posed_angle("femur.L") > 0.05,
			"jump should continue folding from the captured ascent pose"
		);
		Ok(())
	}
}
