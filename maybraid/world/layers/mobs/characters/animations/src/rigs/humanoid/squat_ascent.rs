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
		let lead = self.shoulder_lead(progress);
		if lead.abs() > f32::EPSILON {
			pose.apply_arm(Side::Left, lead, 0.0, 0.0, 0.0, 0.0);
			pose.apply_arm(Side::Right, lead, 0.0, 0.0, 0.0, 0.0);
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
	fn jump_interrupt_from_visible_mid_ascent() -> anyhow::Result<()> {
		use crate::animations::{JumpSegment, TwoFootedJump, Transition};
		use character_rigs::authoring::ArmatureOffset;

		let mut rig = HumanoidV0Rig::for_clip_test();
		SquatAscent::default().apply(&mut rig, 0.4);
		let from_pose = rig.pose.clone();
		let jump = TwoFootedJump::default();
		let lengths = rig.segment_lengths;
		let windup = jump.timings(lengths).squat_descent_duration * 0.5;
		let transition = Transition::from_visible(jump.clone(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, windup, 0.0);
		assert_eq!(jump.segment(lengths, windup).0, JumpSegment::Squat);
		assert!(
			rig.posed_angle("femur.L") > 0.05,
			"jump should continue folding from the captured ascent pose"
		);
		Ok(())
	}
}
