//! Humanoid mapping for [`FacePalm`](crate::animations::FacePalm).

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::FacePalm;
use crate::Animation;

impl Animation<HumanoidV0Rig> for FacePalm {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let amount = self.strike_amount(progress);
		if amount < 1e-5 {
			return;
		}

		let mut pose = HumanoidPose::default();
		pose.apply_spine_pitch(self.spine_lean_back(progress));

		let tilt = self.neck_tilt_back(progress);
		pose.apply_neck_twisted(0.0, 0.0, tilt * 0.35, 0.0, 0.0, tilt * 0.65);

		let arm = pose.arm_mut(self.side);
		arm.shoulder_lift += self.shoulder_lift(progress);
		arm.elbow_flexion += self.elbow_flex(progress);
		arm.aim = Some(ArmAim {
			along: self.humerus_along(progress),
			roll: self.palm_roll() * self.side.sign(),
		});

		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;

	use super::*;
	use crate::animations::{FacePalm, Transition};
	use crate::Animation;
	use character_rigs::authoring::ArmatureOffset;

	const PEAK: f32 = 0.48;
	const FOREARM_LEN: f32 = 0.48;
	const SAMPLE_DT: f32 = 1.0 / 60.0;

	fn hand_tip(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		let bone = format!("forearm.{}", side.suffix());
		rig.character_point(&bone) + rig.character_length(&bone) * FOREARM_LEN
	}

	fn apply_palm(rig: &mut HumanoidV0Rig, side: Side, progress: f32) {
		FacePalm::new(side).apply_for(rig, progress);
	}

	#[test]
	fn face_palm_posed_evidence_on_clip_test_rest() -> anyhow::Result<()> {
		let rest = HumanoidV0Rig::for_clip_test();
		let rest_hand = hand_tip(&rest, Side::Right);

		let mut peak = HumanoidV0Rig::for_clip_test();
		apply_palm(&mut peak, Side::Right, PEAK);
		let hand = hand_tip(&peak, Side::Right);

		assert!(
			rest_hand.x.abs() - hand.x.abs() > 0.75,
			"palm crosses inboard toward midline, hand {hand:?} rest {rest_hand:?}"
		);
		assert!(hand.x.abs() < 0.22, "palm centers on the brow, hand.x={}", hand.x);
		assert!(
			hand.z > rest_hand.z + 0.25,
			"palm comes forward toward the face, hand {hand:?} rest {rest_hand:?}"
		);
		assert!(
			hand.y > rest_hand.y - 0.05,
			"palm stays near head height, hand {hand:?} rest {rest_hand:?}"
		);

		for progress in [0.0, 0.12, PEAK, 0.72, 1.0] {
			let mut posed = HumanoidV0Rig::for_clip_test();
			apply_palm(&mut posed, Side::Right, progress);
			let opp = Side::Left;
			for bone in [format!("humerus.{}", opp.suffix()), format!("forearm.{}", opp.suffix())] {
				assert!(
					posed.posed_angle(&bone) < 1e-4,
					"progress {progress}: opposite {bone} at rest"
				);
			}
			for bone in ["femur.L", "femur.R", "shin.L", "shin.R"] {
				assert!(posed.posed_angle(bone) < 0.02, "progress {progress}: leg {bone} at rest");
			}
		}

		Ok(())
	}

	#[test]
	fn face_palm_repeatability_and_resample_do_not_accumulate() -> anyhow::Result<()> {
		let palm = FacePalm::default();
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		palm.apply_for(&mut once, PEAK);
		palm.apply_for(&mut twice, PEAK);
		palm.apply_for(&mut twice, PEAK);
		for bone in once.animation_bone_names() {
			let a = once.rotation(bone);
			let b = twice.rotation(bone);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample drift on {bone}");
		}
		Ok(())
	}

	#[test]
	fn face_palm_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let palm = FacePalm::default();
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		palm.apply_for(&mut entry, 0.0);
		palm.apply_for(&mut exit, 1.0);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn face_palm_mirrors_hand_placement() -> anyhow::Result<()> {
		let mut right_rig = HumanoidV0Rig::for_clip_test();
		let mut left_rig = HumanoidV0Rig::for_clip_test();
		apply_palm(&mut right_rig, Side::Right, PEAK);
		apply_palm(&mut left_rig, Side::Left, PEAK);

		let r_tip = hand_tip(&right_rig, Side::Right);
		let l_tip = hand_tip(&left_rig, Side::Left);
		assert!((r_tip.x + l_tip.x).abs() < 0.06, "mirrored lateral {r_tip:?} {l_tip:?}");
		assert!((r_tip.y - l_tip.y).abs() < 0.06, "matched height {r_tip:?} {l_tip:?}");
		assert!((r_tip.z - l_tip.z).abs() < 0.06, "matched depth {r_tip:?} {l_tip:?}");
		Ok(())
	}

	#[test]
	fn face_palm_samples_stably_at_sixty_hertz() -> anyhow::Result<()> {
		let palm = FacePalm::default();
		let mut prev_tip = {
			let mut rig = HumanoidV0Rig::for_clip_test();
			palm.apply_for(&mut rig, 0.0);
			hand_tip(&rig, Side::Right)
		};
		for step in 1..59 {
			let t = step as f32 * SAMPLE_DT;
			let mut rig = HumanoidV0Rig::for_clip_test();
			palm.apply_for(&mut rig, t);
			let tip = hand_tip(&rig, Side::Right);
			let jump = (tip - prev_tip).length();
			let limit = if step == 1 { 1.85 } else { 0.09 };
			assert!(
				jump < limit,
				"60 Hz step {step} at t={t:.3} jumped {jump:.4} m (limit {limit})"
			);
			prev_tip = tip;
		}
		Ok(())
	}

	#[test]
	fn face_palm_transition_from_idle_blends_toward_contact() -> anyhow::Result<()> {
		use crate::animations::Idle;

		let mut from_idle = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut from_idle, 0.0);
		let idle_pose = from_idle.pose.clone();
		let rest_hand = hand_tip(&from_idle, Side::Right);

		let into =
			Transition::from_visible(FacePalm::default(), idle_pose, ArmatureOffset::IDENTITY);
		into.apply(&mut from_idle, PEAK, 0.5);

		let blended = hand_tip(&from_idle, Side::Right);
		let full = {
			let mut rig = HumanoidV0Rig::for_clip_test();
			apply_palm(&mut rig, Side::Right, PEAK);
			hand_tip(&rig, Side::Right)
		};
		let toward = (blended.y - rest_hand.y).max(0.0);
		let full_delta = (full.y - rest_hand.y).max(0.0);
		assert!(
			toward > full_delta * 0.2 && toward < full_delta * 0.9,
			"50% blend between idle and peak, Δy {toward:.4} vs full {full_delta:.4}"
		);
		Ok(())
	}

	#[test]
	fn face_palm_interruption_mid_raise_blends_without_pop() -> anyhow::Result<()> {
		let palm = FacePalm::default();
		let mut partial = HumanoidV0Rig::for_clip_test();
		palm.apply_for(&mut partial, 0.14);
		let captured = partial.pose.clone();
		let partial_tip = hand_tip(&partial, Side::Right);

		let transition = Transition::from_visible(palm, captured, ArmatureOffset::IDENTITY);
		transition.apply(&mut partial, PEAK, 0.35);

		let blended_tip = hand_tip(&partial, Side::Right);
		let full = {
			let mut rig = HumanoidV0Rig::for_clip_test();
			apply_palm(&mut rig, Side::Right, PEAK);
			hand_tip(&rig, Side::Right)
		};
		let toward = (blended_tip - partial_tip).length();
		let span = (full - partial_tip).length();
		assert!(
			toward > span * 0.05 && toward < span * 0.95,
			"interrupted blend lands between partial raise and peak"
		);
		Ok(())
	}
}
