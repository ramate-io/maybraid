//! Humanoid mapping for [`StompBeat`](crate::animations::StompBeat).

use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::StompBeat;
use crate::Animation;

impl Animation<HumanoidV0Rig> for StompBeat {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		if self.motion_amount(progress) < 1e-5 {
			return;
		}

		let stomp_side = self.stomping_side();
		let support_side = stomp_side.opposite();
		let pump_side = self.pump_side();

		let mut pose = HumanoidPose::default();
		pose.apply_leg(
			stomp_side,
			self.stomping_hip_flexion(progress),
			self.stomping_knee_flexion(progress),
		);
		pose.apply_leg(support_side, 0.0, self.support_knee_flexion(progress));
		pose.apply_spine_pitch(self.torso_dip(progress));

		let along = self.pump_humerus_along(progress);
		if along != Vec3::ZERO {
			let arm = pose.arm_mut(pump_side);
			arm.elbow_flexion += self.pump_elbow(progress);
			arm.aim = Some(ArmAim { along, roll: self.pump_roll() * pump_side.sign() });
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
	use crate::animations::stomp_beat::{IMPACT_CENTER, LIFT_CENTER};
	use crate::Animation;

	const SAMPLE_PROGRESS: [f32; 8] = [0.0, 0.14, 0.26, 0.33, 0.40, 0.50, 0.72, 1.0];

	fn shin_tip(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		rig.character_point(&format!("shin.{}", side.suffix()))
	}

	fn forearm_tip(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		rig.character_point(&format!("forearm.{}", side.suffix()))
	}

	fn peak_lift() -> f32 {
		LIFT_CENTER
	}

	fn peak_impact() -> f32 {
		IMPACT_CENTER
	}

	#[test]
	fn stomp_beat_posed_evidence_on_clip_test_rest() -> anyhow::Result<()> {
		let stomp = StompBeat::default().with_side(Side::Right);
		let rest = HumanoidV0Rig::for_clip_test();
		let rest_stomp_shin = shin_tip(&rest, Side::Right);

		let mut lifted = HumanoidV0Rig::for_clip_test();
		stomp.apply_for(&mut lifted, peak_lift());
		let lifted_shin = shin_tip(&lifted, Side::Right);
		let lift_delta = lifted_shin - rest_stomp_shin;
		assert!(lift_delta.z < -0.18, "knee folds forward on cock, Δ={lift_delta:?}");
		assert!(
			lift_delta.y > 0.04 || lift_delta.length() > 0.22,
			"cocked leg displaces clearly, Δ={lift_delta:?}"
		);

		let mut struck = HumanoidV0Rig::for_clip_test();
		stomp.apply_for(&mut struck, peak_impact());
		let struck_shin = shin_tip(&struck, Side::Right);
		assert!(
			struck_shin.y < lifted_shin.y - 0.08,
			"shin drops on impact {:?} vs lift {:?}",
			struck_shin,
			lifted_shin
		);
		assert!(
			struck_shin.y < rest_stomp_shin.y + 0.04,
			"impact foot near or below rest height {:?} vs {:?}",
			struck_shin,
			rest_stomp_shin
		);

		let pump_delta = forearm_tip(&struck, Side::Left) - forearm_tip(&rest, Side::Left);
		assert!(pump_delta.y < -0.06, "left arm whips down on right stomp, Δ={pump_delta:?}");
		assert!(pump_delta.z > 0.02, "pump carries forward, Δ={pump_delta:?}");

		assert!(
			struck.posed_angle("shin.L") > rest.posed_angle("shin.L") + 0.04,
			"support knee flexes on impact"
		);

		Ok(())
	}

	#[test]
	fn stomp_beat_mirrors_for_left_stomp() -> anyhow::Result<()> {
		let right = StompBeat::default().with_side(Side::Right);
		let left = StompBeat::default().with_side(Side::Left);
		let mut r = HumanoidV0Rig::for_clip_test();
		let mut l = HumanoidV0Rig::for_clip_test();
		right.apply_for(&mut r, peak_impact());
		left.apply_for(&mut l, peak_impact());

		let r_shin = shin_tip(&r, Side::Right);
		let l_shin = shin_tip(&l, Side::Left);
		assert!((r_shin.x + l_shin.x).abs() < 0.04, "mirrored stomp shin {r_shin:?} {l_shin:?}");

		let r_pump = forearm_tip(&r, Side::Left);
		let l_pump = forearm_tip(&l, Side::Right);
		assert!((r_pump.x + l_pump.x).abs() < 0.04, "mirrored pump {r_pump:?} {l_pump:?}");
		assert!((r_pump.y - l_pump.y).abs() < 0.06, "matched pump height");
		Ok(())
	}

	#[test]
	fn stomp_beat_repeatability_and_resample_do_not_accumulate() -> anyhow::Result<()> {
		let stomp = StompBeat::default();
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		stomp.apply_for(&mut once, 0.35);
		stomp.apply_for(&mut twice, 0.35);
		stomp.apply_for(&mut twice, 0.35);
		for bone in once.animation_bone_names() {
			let a = once.rotation(bone);
			let b = twice.rotation(bone);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample drift on {bone}");
		}
		Ok(())
	}

	#[test]
	fn stomp_beat_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let stomp = StompBeat::default();
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		stomp.apply_for(&mut entry, 0.0);
		stomp.apply_for(&mut exit, 1.0);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn stomp_beat_interruption_mid_clip_is_stable() -> anyhow::Result<()> {
		let stomp = StompBeat::default();
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		stomp.apply_for(&mut once, 0.33);
		stomp.apply_for(&mut twice, 0.33);
		stomp.apply_for(&mut twice, 0.33);
		for bone in ["femur.R", "shin.R", "humerus.L", "forearm.L", "lumbar"] {
			let delta = once.rotation(bone).angle_between(twice.rotation(bone));
			assert!(delta < 1e-3, "{bone} drifted {delta}");
		}
		Ok(())
	}

	#[test]
	fn stomp_beat_unwritten_bones_stay_at_rest_at_peak_lift() -> anyhow::Result<()> {
		let stomp = StompBeat::default().with_side(Side::Right);
		let mut posed = HumanoidV0Rig::for_clip_test();
		stomp.apply_for(&mut posed, peak_lift());

		for bone in ["humerus.R", "forearm.R"] {
			assert!(posed.posed_angle(bone) < 0.02, "stomping-side arm {bone} stays quiet at lift");
		}
		for bone in ["humerus.L", "forearm.L"] {
			let angle = posed.posed_angle(bone);
			assert!(angle < 0.05, "pump arm {bone} not active yet at lift (angle={angle})");
		}
		Ok(())
	}

	#[test]
	fn stomp_beat_samples_track_timing() -> anyhow::Result<()> {
		let stomp = StompBeat::default();
		let rest = HumanoidV0Rig::for_clip_test();
		let rest_tip = shin_tip(&rest, Side::Right);

		for progress in SAMPLE_PROGRESS {
			let mut posed = HumanoidV0Rig::for_clip_test();
			stomp.apply_for(&mut posed, progress);
			let tip = shin_tip(&posed, Side::Right);
			if progress == 0.0 || progress >= 0.99 {
				assert!(
					(tip - rest_tip).length() < 1e-3,
					"progress {progress} equals rest, tip {tip:?}"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn stomp_beat_transition_from_visible_at_mid_lift_is_stable() -> anyhow::Result<()> {
		use crate::animations::{Idle, Transition};
		use character_rigs::authoring::ArmatureOffset;

		let mut lifted = HumanoidV0Rig::for_clip_test();
		StompBeat::default().apply_for(&mut lifted, peak_lift());
		let from_pose = lifted.pose.clone();
		let lifted_shin = shin_tip(&lifted, Side::Right);

		let transition =
			Transition::from_visible(StompBeat::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut lifted, peak_impact(), 0.5);

		assert!(
			shin_tip(&lifted, Side::Right).y < lifted_shin.y,
			"blend toward impact lowers the stomping foot"
		);

		let mut from_idle = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut from_idle, 0.0);
		let idle_pose = from_idle.pose.clone();
		let into_lift =
			Transition::from_visible(StompBeat::default(), idle_pose, ArmatureOffset::IDENTITY);
		into_lift.apply(&mut from_idle, peak_lift(), 0.5);

		let half_lift_y = shin_tip(&from_idle, Side::Right).y;
		let full_lift = {
			let mut full = HumanoidV0Rig::for_clip_test();
			StompBeat::default().apply_for(&mut full, peak_lift());
			shin_tip(&full, Side::Right).y
		};
		let rest_y = shin_tip(&HumanoidV0Rig::for_clip_test(), Side::Right).y;
		assert!(
			half_lift_y > rest_y + (full_lift - rest_y) * 0.25,
			"50% blend lifts the foot between idle and cocked"
		);
		Ok(())
	}
}
