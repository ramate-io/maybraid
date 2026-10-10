//! Humanoid mapping for [`FistPump`](crate::animations::FistPump).

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

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
	use character_rigs::Side;

	use super::*;
	use crate::Animation;

	fn peak() -> f32 {
		0.43
	}

	fn bone_segment(rig: &HumanoidV0Rig, name: &str) -> f32 {
		let id = rig.binding.definition.id(name).expect(name);
		rig.binding.effective_rest.local[id.index()].translation.length()
	}

	fn bone_tail(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		let origin = rig.character_point(name);
		origin + rig.character_length(name) * bone_segment(rig, name)
	}

	fn fist(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		bone_tail(rig, &format!("forearm.{}", side.suffix()))
	}

	fn elbow(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		rig.character_point(&format!("forearm.{}", side.suffix()))
	}

	fn shoulder(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		rig.character_point(&format!("humerus.{}", side.suffix()))
	}

	fn head_top(rig: &HumanoidV0Rig) -> f32 {
		bone_tail(rig, "upper_neck").y
	}

	fn chamber_progress() -> f32 {
		0.09
	}

	fn overhead_peak_progress(pump: &FistPump) -> f32 {
		let mut best = peak();
		let mut best_y = f32::NEG_INFINITY;
		for step in 0..=48 {
			let t = (step as f32 / 48.0) * 0.62;
			let mut rig = HumanoidV0Rig::for_clip_test();
			pump.apply(&mut rig, t);
			let y = fist(&rig, pump.side).y;
			if y > best_y {
				best_y = y;
				best = t;
			}
		}
		best
	}

	#[derive(Debug)]
	struct Snap {
		fist: Vec3,
		elbow: Vec3,
		shoulder: Vec3,
		head_y: f32,
	}

	fn snapshot(rig: &HumanoidV0Rig, side: Side) -> Snap {
		Snap {
			fist: fist(rig, side),
			elbow: elbow(rig, side),
			shoulder: shoulder(rig, side),
			head_y: head_top(rig),
		}
	}

	fn assert_mirrored(right: &Snap, left: &Snap) {
		assert!((right.fist.x + left.fist.x).abs() < 0.06, "fist mirror {right:?} {left:?}");
		assert!((right.fist.y - left.fist.y).abs() < 0.06, "fist height {right:?} {left:?}");
		assert!((right.fist.z - left.fist.z).abs() < 0.06, "fist depth {right:?} {left:?}");
		assert!((right.elbow.x + left.elbow.x).abs() < 0.06, "elbow mirror");
		assert!((right.shoulder.x + left.shoulder.x).abs() < 0.06, "shoulder mirror");
	}

	#[test]
	fn fist_pump_character_space_pose_contract() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let pump = FistPump::default().with_side(side);
			let rest = snapshot(&HumanoidV0Rig::for_clip_test(), side);

			let mut chamber_rig = HumanoidV0Rig::for_clip_test();
			pump.apply(&mut chamber_rig, chamber_progress());
			let chamber = snapshot(&chamber_rig, side);
			assert!(chamber.fist.z < rest.fist.z - 0.3, "chamber pulls back, {chamber:?}");

			let mut hold_rig = HumanoidV0Rig::for_clip_test();
			pump.apply(&mut hold_rig, peak());
			let hold_y = fist(&hold_rig, side).y;
			assert!(
				chamber.fist.y < hold_y - 0.2,
				"chamber precedes overhead hold height, {chamber:?} hold_y {hold_y}"
			);

			let overhead_t = overhead_peak_progress(&pump);
			let mut peak_rig = HumanoidV0Rig::for_clip_test();
			pump.apply(&mut peak_rig, overhead_t);
			let snap = snapshot(&peak_rig, side);
			assert!(snap.fist.y > snap.head_y + 0.35, "overhead fist clears head, {snap:?}");
			assert!(
				snap.fist.x.signum() == rest.fist.x.signum(),
				"fist must not cross midline at snap, {snap:?} rest {rest:?}"
			);

			let hold = snapshot(&hold_rig, side);
			assert!(hold.fist.y > hold.head_y + 0.3, "hold keeps fist overhead, {hold:?}");
			assert!(
				hold.fist.x.signum() == rest.fist.x.signum(),
				"hold stays on pumping side, {hold:?}"
			);
			assert!(
				(hold.fist.x - hold.shoulder.x).abs() < 0.35,
				"fist sits near its shoulder column, {hold:?}"
			);

			let mut end_rig = HumanoidV0Rig::for_clip_test();
			pump.apply(&mut end_rig, 1.0);
			let end = snapshot(&end_rig, side);
			assert!((end.fist - rest.fist).length() < 1e-3, "end returns to rest {end:?}");
		}

		let mut right_rig = HumanoidV0Rig::for_clip_test();
		let mut left_rig = HumanoidV0Rig::for_clip_test();
		FistPump::default().with_side(Side::Right).apply(&mut right_rig, peak());
		FistPump::default().with_side(Side::Left).apply(&mut left_rig, peak());
		assert_mirrored(&snapshot(&right_rig, Side::Right), &snapshot(&left_rig, Side::Left));
		Ok(())
	}

	#[test]
	fn fist_pump_hold_uses_bent_elbow_distinct_from_extended_wave() -> anyhow::Result<()> {
		let pump = FistPump::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::for_clip_test();
		pump.apply(&mut rig, peak());
		// VictoryWave peak (~0.55) lands near ~0.6 rad; the pump stays clearly more flexed.
		assert!(
			rig.posed_angle("forearm.R") > 0.68,
			"bent pump elbow, got {}",
			rig.posed_angle("forearm.R")
		);
		Ok(())
	}

	#[test]
	fn fist_pump_transition_from_visible_mid_pump_blends_from_rest() -> anyhow::Result<()> {
		use crate::animations::Transition;
		use character_rigs::authoring::ArmatureOffset;

		let rest = HumanoidV0Rig::for_clip_test();
		let from_pose = rest.pose.clone();
		let mut rig = HumanoidV0Rig::for_clip_test();
		let transition =
			Transition::from_visible(FistPump::default(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.38, 0.0);
		assert!(
			rig.rotation("forearm.R").dot(rest.rotation("forearm.R")).abs() > 1.0 - 1e-4,
			"transition weight 0 keeps captured rest pose"
		);
		transition.apply(&mut rig, 0.38, 1.0);
		let mut target = HumanoidV0Rig::for_clip_test();
		FistPump::default().apply(&mut target, 0.38);
		assert!(
			rig.rotation("forearm.R").dot(target.rotation("forearm.R")).abs() > 1.0 - 1e-4,
			"transition weight 1 matches mid-pump target"
		);
		Ok(())
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
			let rest = HumanoidV0Rig::for_clip_test();
			let mut posed = HumanoidV0Rig::for_clip_test();
			FistPump::default().with_side(side).apply(&mut posed, peak());

			let rest_tip = fist(&rest, side);
			let posed_tip = fist(&posed, side);
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

		let r_tip = fist(&r, Side::Right);
		let l_tip = fist(&l, Side::Left);
		assert!((r_tip.x + l_tip.x).abs() < 0.05, "mirrored placement {r_tip:?} {l_tip:?}");
		assert!((r_tip.y - l_tip.y).abs() < 0.08, "matched height {r_tip:?} {l_tip:?}");
		Ok(())
	}

	#[test]
	fn fist_pump_leaves_opposite_arm_at_rest() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let other = side.opposite();
			let pump = FistPump::default().with_side(side);
			let mut posed = HumanoidV0Rig::for_clip_test();
			pump.apply(&mut posed, peak());

			for bone in
				[format!("humerus.{}", other.suffix()), format!("forearm.{}", other.suffix())]
			{
				assert!(posed.posed_angle(&bone) < 1e-4, "opposite {bone} stays at rest");
			}
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
