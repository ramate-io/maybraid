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
		arm.shoulder_lift += self.shoulder_lift(progress);
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
	const FOREARM_LEN: f32 = 0.48;
	const EXTEND_END: f32 = 0.22;
	const BECKON_END: f32 = 0.76;

	struct ArmSample {
		hand: Vec3,
		elbow: Vec3,
		shoulder: Vec3,
	}

	fn bone_tail(rig: &HumanoidV0Rig, name: &str, length: f32) -> Vec3 {
		rig.character_point(name) + rig.character_length(name) * length
	}

	fn hand_tip(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		let bone = format!("forearm.{}", side.suffix());
		bone_tail(rig, &bone, FOREARM_LEN)
	}

	fn elbow_joint(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		rig.character_point(&format!("forearm.{}", side.suffix()))
	}

	fn shoulder_joint(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		rig.character_point(&format!("shoulder.{}", side.suffix()))
	}

	fn sample_arm(beckon: &Beckon, progress: f32, side: Side) -> ArmSample {
		let mut rig = HumanoidV0Rig::for_clip_test();
		beckon.apply(&mut rig, progress);
		ArmSample {
			hand: hand_tip(&rig, side),
			elbow: elbow_joint(&rig, side),
			shoulder: shoulder_joint(&rig, side),
		}
	}

	fn curl_hold_extremes(beckon: &Beckon) -> (f32, f32) {
		let hold_start = EXTEND_END + 0.01;
		let hold_end = BECKON_END - 0.02;
		let mut flat_p = hold_start;
		let mut peak_p = hold_start;
		let mut flat_abs = f32::MAX;
		let mut peak_abs = f32::MIN;
		for i in 0..200 {
			let p = hold_start + (hold_end - hold_start) * (i as f32 / 199.0);
			let w = beckon.curl_wave(p).abs();
			if w < flat_abs {
				flat_abs = w;
				flat_p = p;
			}
			if w > peak_abs {
				peak_abs = w;
				peak_p = p;
			}
		}
		(flat_p, peak_p)
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
	fn beckon_posed_evidence_on_each_side() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let beckon = Beckon::new(side);
			let rest = sample_arm(&beckon, 0.0, side);
			let reach = sample_arm(&beckon, PEAK, side);
			let (curl_flat_p, curl_peak_p) = curl_hold_extremes(&beckon);
			let curl_flat = sample_arm(&beckon, curl_flat_p, side);
			let curl_peak = sample_arm(&beckon, curl_peak_p, side);
			let end = sample_arm(&beckon, 1.0, side);

			assert!(
				(reach.hand - rest.hand).length() > 0.4,
				"{side:?} hand leaves rest at full reach"
			);
			assert!(
				reach.hand.z > rest.hand.z + 0.35,
				"{side:?} hand reaches fight-forward, {:?}",
				reach.hand
			);
			assert!(
				reach.hand.y > reach.shoulder.y - 0.05 && reach.hand.y < 1.08,
				"{side:?} hand near chest–shoulder height, {:?}",
				reach.hand
			);
			for label in ["reach", "curl_flat", "curl_peak"] {
				let hand = match label {
					"reach" => reach.hand,
					"curl_flat" => curl_flat.hand,
					"curl_peak" => curl_peak.hand,
					_ => unreachable!(),
				};
				assert!(
					hand.x.signum() == side.sign(),
					"{side:?} {label} hand stays on its side, {hand:?}"
				);
			}
			let curl_delta = curl_peak.hand - curl_flat.hand;
			let curl_dist = curl_delta.length();
			assert!(curl_dist > 0.05, "{side:?} curl pulse must read (>{curl_dist} m)");
			assert!(
				curl_delta.z < -0.02,
				"{side:?} curl pulls hand back toward body, Δ{curl_delta:?}"
			);
			assert!(curl_delta.y > 0.02, "{side:?} curl lifts the hand, Δ{curl_delta:?}");

			assert!(
				end.hand.distance(rest.hand) < 1e-3 && end.elbow.distance(rest.elbow) < 1e-3,
				"{side:?} end pose matches rest"
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

		let r_tip = hand_tip(&right_rig, Side::Right);
		let l_tip = hand_tip(&left_rig, Side::Left);
		assert!((r_tip.x + l_tip.x).abs() < 0.05, "mirrored lateral {r_tip:?} {l_tip:?}");
		assert!((r_tip.y - l_tip.y).abs() < 0.06, "matched height {r_tip:?} {l_tip:?}");
		assert!((r_tip.z - l_tip.z).abs() < 0.06, "matched forward reach {r_tip:?} {l_tip:?}");
		Ok(())
	}

	#[test]
	fn beckon_leaves_opposite_arm_at_rest() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let beckon = Beckon::new(side);
			let mut posed = HumanoidV0Rig::for_clip_test();
			beckon.apply(&mut posed, PEAK);
			let opp = side.opposite();
			for bone in [format!("humerus.{}", opp.suffix()), format!("forearm.{}", opp.suffix())] {
				assert!(posed.posed_angle(&bone) < 1e-4, "{side:?} opposite {bone} stays at rest");
			}
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
	fn beckon_transition_from_visible_mid_pulse() -> anyhow::Result<()> {
		use crate::animations::{Idle, Transition};
		use character_rigs::authoring::ArmatureOffset;

		let beckon = Beckon::new(Side::Right);
		let mid = 0.42;
		let mut full = HumanoidV0Rig::for_clip_test();
		beckon.apply_for(&mut full, mid);
		let full_hand = hand_tip(&full, Side::Right);

		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut rig, 0.0);
		let from_pose = rig.pose.clone();
		let rest_hand = hand_tip(&rig, Side::Right);
		let transition = Transition::from_visible(beckon, from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, mid, 0.5);
		let blended_hand = hand_tip(&rig, Side::Right);

		assert!(
			blended_hand.z > rest_hand.z + 0.08 && blended_hand.z < full_hand.z,
			"mid-pulse transition blends forward reach, rest {rest_hand:?} blend {blended_hand:?} full {full_hand:?}"
		);
		assert!(
			blended_hand.x.signum() == Side::Right.sign(),
			"gesturing hand stays on its side during transition"
		);
		for bone in ["femur.L", "femur.R", "shin.L", "shin.R"] {
			let angle = rig.posed_angle(bone);
			assert!(angle < 0.05, "unwritten {bone} near rest during transition");
		}
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
