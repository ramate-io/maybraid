//! Humanoid mapping for [`FistPump`](crate::animations::FistPump).

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::FistPump;
use crate::Animation;

impl Animation<HumanoidV0Rig> for FistPump {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let amount = self.pump_amount(progress);
		if amount < 1e-5 {
			return;
		}

		let mut pose = HumanoidPose::default();
		let pump_side = self.side;

		pose.apply_root(self.root_lean_back(progress));
		pose.apply_leg(pump_side, 0.0, self.stance_knee(progress));

		let arm = pose.arm_mut(pump_side);
		arm.shoulder_forward += self.shoulder_windup(progress);
		arm.elbow_flexion += self.elbow_flex(progress);
		arm.aim = Some(ArmAim {
			along: self.humerus_along(progress),
			roll: self.pump_roll() * pump_side.sign(),
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

	fn peak() -> f32 {
		0.43
	}

	fn forearm_tip(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		rig.character_point(&format!("forearm.{}", side.suffix()))
	}

	#[test]
	fn fist_pump_repeatability() -> anyhow::Result<()> {
		let pump = FistPump::default();
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		pump.apply(&mut a, peak());
		pump.apply(&mut b, peak());
		for bone in ["humerus.R", "forearm.R", "femur.R", "shin.R"] {
			let ra = a.rotation(bone);
			let rb = b.rotation(bone);
			assert!(ra.dot(rb).abs() > 1.0 - 1e-5, "{bone} must match");
		}
		Ok(())
	}

	#[test]
	fn fist_pump_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let pump = FistPump::default();
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		pump.apply_for(&mut entry, 0.0);
		pump.apply_for(&mut exit, 1.0);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn fist_pump_interruption_mid_clip_is_stable() -> anyhow::Result<()> {
		let pump = FistPump::default();
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		pump.apply_for(&mut once, 0.38);
		pump.apply_for(&mut twice, 0.38);
		pump.apply_for(&mut twice, 0.38);
		for bone in ["humerus.R", "forearm.R"] {
			let delta = once.rotation(bone).angle_between(twice.rotation(bone));
			assert!(delta < 1e-3, "{bone} drifted {delta}");
		}
		Ok(())
	}

	#[test]
	fn fist_pump_raises_forearm_tip_overhead() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let bone = format!("forearm.{}", side.suffix());
			let rest = HumanoidV0Rig::for_clip_test();
			let mut posed = HumanoidV0Rig::for_clip_test();
			FistPump::default().with_side(side).apply(&mut posed, peak());

			let rest_tip = rest.character_point(&bone);
			let posed_tip = forearm_tip(&posed, side);
			assert!(
				posed_tip.y > rest_tip.y + 0.22,
				"{side:?} fist rises overhead, {posed_tip:?} vs {rest_tip:?}"
			);
			assert!(
				posed_tip.x.signum() == rest_tip.x.signum(),
				"{side:?} fist must not cross midline, {posed_tip:?} vs {rest_tip:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn fist_pump_mirrors_forearm_height_for_both_sides() -> anyhow::Result<()> {
		let right = FistPump::default().with_side(Side::Right);
		let left = FistPump::default().with_side(Side::Left);
		let mut r = HumanoidV0Rig::for_clip_test();
		let mut l = HumanoidV0Rig::for_clip_test();
		right.apply(&mut r, peak());
		left.apply(&mut l, peak());

		let r_tip = r.character_point("forearm.R");
		let l_tip = l.character_point("forearm.L");
		assert!((r_tip.x + l_tip.x).abs() < 0.05, "mirrored placement {r_tip:?} {l_tip:?}");
		assert!((r_tip.y - l_tip.y).abs() < 0.08, "matched height {r_tip:?} {l_tip:?}");
		Ok(())
	}

	#[test]
	fn fist_pump_leaves_opposite_arm_at_rest() -> anyhow::Result<()> {
		let pump = FistPump::default().with_side(Side::Right);
		let mut posed = HumanoidV0Rig::for_clip_test();
		pump.apply(&mut posed, peak());

		for bone in ["humerus.L", "forearm.L"] {
			assert!(posed.posed_angle(bone) < 1e-4, "opposite {bone} stays at rest");
		}
		Ok(())
	}

	#[test]
	fn fist_pump_leaves_opposite_leg_at_rest() -> anyhow::Result<()> {
		let pump = FistPump::default().with_side(Side::Right);
		let mut posed = HumanoidV0Rig::for_clip_test();
		pump.apply(&mut posed, peak());

		for bone in ["femur.L", "shin.L"] {
			assert!(posed.posed_angle(bone) < 0.02, "unwritten opposite leg {bone}");
		}
		Ok(())
	}

	#[test]
	fn fist_pump_elbow_flexes_at_peak() -> anyhow::Result<()> {
		let pump = FistPump::default().with_side(Side::Right);
		let rest = HumanoidV0Rig::for_clip_test();
		let mut posed = HumanoidV0Rig::for_clip_test();
		pump.apply(&mut posed, peak());

		assert!(
			posed.posed_angle("forearm.R") > rest.posed_angle("forearm.R") + 0.35,
			"pumping elbow leaves rest"
		);
		Ok(())
	}

	#[test]
	fn fist_pump_humerus_aims_up_in_character_space() -> anyhow::Result<()> {
		let pump = FistPump::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::for_clip_test();
		pump.apply(&mut rig, peak());

		let along = pump.humerus_along(peak());
		let humerus_dir = rig.character_length("humerus.R");
		assert!(humerus_dir.dot(along) > 0.75, "aim {along:?} humerus {humerus_dir:?}");
		assert!(humerus_dir.y > 0.45, "humerus points up, {humerus_dir:?}");
		Ok(())
	}
}
