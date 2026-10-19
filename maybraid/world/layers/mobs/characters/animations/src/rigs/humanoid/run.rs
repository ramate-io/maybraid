use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Run, UprightRun};
use crate::rigs::humanoid::gait_coordination::GaitCoordination;
use crate::rigs::humanoid::gait_knee::lerp_swing_knee;
use crate::rigs::humanoid::write_masks::{debug_assert_pose_within_mask, run_write_mask};
use crate::Animation;

impl Run {
	/// Authored semantic pose at `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		UprightRun::from_run(self).sample_pose(progress)
	}
}

impl Animation<HumanoidV0Rig> for Run {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let pose = self.sample_pose(progress);
		debug_assert_pose_within_mask(&pose, run_write_mask(), "run");
		rig.apply_masked_pose(&pose, run_write_mask());
	}
}

impl UprightRun {
	/// Authored semantic pose at `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let coord = GaitCoordination::from_progress(progress);
		let phase = coord.phase();
		let left_arm_swing = -GaitCoordination::arm_swing(phase);
		let right_arm_swing = GaitCoordination::arm_swing(phase + 0.5);

		coord.apply_spine_counter(&mut pose, self.torso_lean, self.torso_turn, self.side_lean);
		apply_leg(&mut pose, Side::Left, coord, -1.0, self);
		apply_leg(&mut pose, Side::Right, coord, 1.0, self);
		// Both elbows share one flexion sign. Opposite arm_down values are a hang bias.
		apply_run_arm(&mut pose, Side::Left, left_arm_swing, coord, -self.arm_down, self);
		apply_run_arm(&mut pose, Side::Right, right_arm_swing, coord, self.arm_down, self);
		coord.apply_head(
			&mut pose,
			self.torso_lean,
			self.torso_turn,
			self.side_lean,
			self.head_stabilize,
		);
		pose
	}
}

impl Animation<HumanoidV0Rig> for UprightRun {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let pose = self.sample_pose(progress);
		debug_assert_pose_within_mask(&pose, run_write_mask(), "upright_run");
		rig.apply_masked_pose(&pose, run_write_mask());
	}
}

fn apply_leg(
	pose: &mut HumanoidPose,
	side: Side,
	coord: GaitCoordination,
	lift_sign: f32,
	run: &UprightRun,
) {
	let cycle = coord.phase();
	let phase = if side == Side::Left { cycle } else { cycle + 0.5 };
	let swing = GaitCoordination::thigh_swing(phase);
	// Same coordinated yaw as walk. Opposite-sign turn was measured as shared
	// hip Z (no pelvic twist) and is what made the run read like a postcard.
	coord.apply_pelvis_yaw_and_bob(
		pose,
		side,
		swing,
		lift_sign,
		run.hip_swing,
		run.hip_lift,
		run.body_bob,
	);
	let leg = pose.leg_mut(side);
	leg.hip_flexion = swing * run.stride;
	leg.knee_flexion = knee_flex(phase, run) - run.knee_extended;
	leg.pelvis_flexion = swing.max(0.0) * run.foot_roll;
}

fn apply_run_arm(
	pose: &mut HumanoidPose,
	side: Side,
	arm_swing_value: f32,
	coord: GaitCoordination,
	humerus_flex: f32,
	run: &UprightRun,
) {
	coord.apply_arm_across(
		pose,
		side,
		arm_swing_value,
		humerus_flex,
		run.shoulder_swing,
		run.shoulder_lift,
		run.humerus_swing_scale,
		run.arm_across,
		GaitCoordination::elbow_flex(
			arm_swing_value,
			coord.phase(),
			-1.0,
			run.elbow_bend,
			run.elbow_pump,
			run.elbow_cycle,
		),
	);
}

fn knee_flex(leg_phase: f32, run: &UprightRun) -> f32 {
	let p = leg_phase.fract();
	if p < 0.5 {
		let t = p * 2.0;
		run.knee_neutral + (t * std::f32::consts::PI).sin() * (run.knee_extended - run.knee_neutral)
	} else {
		lerp_swing_knee(p, run.knee_extended, run.knee_contracted)
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn assert_pose_matches_at_phases(phases: &[f32]) {
		for &phase in phases {
			let mut from_run = HumanoidV0Rig::imported();
			let mut from_upright = HumanoidV0Rig::imported();
			Run::default().apply(&mut from_run, phase);
			UprightRun::default().apply(&mut from_upright, phase);

			for name in from_run.animation_bone_names() {
				let run_rot = from_run.rotation(name);
				let upright_rot = from_upright.rotation(name);
				assert!(
					run_rot.dot(upright_rot).abs() > 1.0 - 1e-5,
					"rotation mismatch on {name} at {phase}"
				);
			}
		}
	}

	#[test]
	fn run_delegates_to_upright_default() {
		assert_pose_matches_at_phases(&[0.0, 0.25, 0.5, 0.75]);
	}

	#[test]
	fn run_flexes_the_left_femur_sagittally() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("femur.L") > 0.2, "stride leaves rest");
	}

	#[test]
	fn run_right_leg_uses_half_cycle_phase_offset() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		UprightRun::default().apply(&mut rig, 0.0);

		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!((left.z - right.z).abs() > 0.05, "legs are out of phase, L={left:?} R={right:?}");
	}

	#[test]
	fn run_applies_knee_flex_to_shin() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.25);
		let extended = rig.posed_angle("shin.L");
		UprightRun::default().apply(&mut rig, 0.75);
		assert!(extended < 0.05, "expected a straight knee at this phase, got {extended}");
		assert!(rig.posed_angle("shin.L") > 0.5, "expected knee tuck later in the stride");
	}

	#[test]
	fn run_applies_elbow_bend_to_forearm() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("forearm.L") > 0.9, "expected elbow bend baseline");
	}

	#[test]
	fn run_applies_shoulder_and_hip_lift() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.0);

		let shoulder = tip(&rig, "shoulder.L");
		let pelvis = tip(&rig, "pelvis.L");
		assert!(shoulder.x.abs() > 0.0, "expected shoulder bounce, got {shoulder:?}");
		assert!(pelvis.x.abs() > 0.0, "expected hip bounce, got {pelvis:?}");
	}

	fn run_at(progress: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Run::default().apply_for(&mut rig, progress);
		rig
	}

	fn shin_tail(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.character_point(name) + rig.character_length(name) * 0.5
	}

	#[test]
	fn run_leans_farther_forward_than_walk() -> anyhow::Result<()> {
		let run = run_at(0.0).rotation("root") * Vec3::Y;
		let mut walk_rig = HumanoidV0Rig::for_clip_test();
		crate::animations::Walk::default().apply_for(&mut walk_rig, 0.0);
		let walk = walk_rig.rotation("root") * Vec3::Y;
		assert!(run.z > 0.14, "run lean goes to +Z, got {run:?}");
		assert!(run.x.abs() < 1e-3, "run lean must not yaw, got {run:?}");
		assert!(run.z > walk.z + 0.04, "run should lean more than walk, run={run:?} walk={walk:?}");
		Ok(())
	}

	#[test]
	fn run_pelvis_yaws_instead_of_sharing_z() -> anyhow::Result<()> {
		let rig = run_at(0.0);
		let hip_l = rig.character_point("femur.L");
		let hip_r = rig.character_point("femur.R");
		assert!(
			hip_l.z - hip_r.z > 0.05,
			"run hips must split in Z (yaw), L={hip_l:?} R={hip_r:?}"
		);
		Ok(())
	}

	#[test]
	fn run_body_bob_is_high_at_midstance() -> anyhow::Result<()> {
		let mid = 0.5
			* (run_at(0.0).character_point("femur.L").y + run_at(0.0).character_point("femur.R").y);
		let contact = 0.5
			* (run_at(0.25).character_point("femur.L").y
				+ run_at(0.25).character_point("femur.R").y);
		assert!(
			mid > contact + 0.015,
			"run midstance should sit above the passing pose, mid={mid} contact={contact}"
		);
		Ok(())
	}

	#[test]
	fn run_shoulders_counter_pelvis_yaw() -> anyhow::Result<()> {
		let rig = run_at(0.0);
		let hip_yaw = rig.character_point("femur.L").z - rig.character_point("femur.R").z;
		let shoulder = rig.rotation("upper_back") * Vec3::Z;
		assert!(
			hip_yaw * shoulder.x < 0.0 && shoulder.x.abs() > 0.02,
			"run torso must counter the pelvis, hip={hip_yaw} upperZ={shoulder:?}"
		);
		Ok(())
	}

	#[test]
	fn run_head_is_more_level_than_the_chest() -> anyhow::Result<()> {
		let rig = run_at(0.0);
		let chest = rig.rotation("root") * Vec3::Y;
		let head = rig.character_length("upper_neck");
		assert!(
			head.z.abs() < chest.z - 0.03,
			"neck should cancel run lean, head={head:?} chest={chest:?}"
		);
		Ok(())
	}

	#[test]
	fn run_foot_roll_pulls_the_trailing_shin_back() -> anyhow::Result<()> {
		let rolled = UprightRun::default();
		let mut flat = UprightRun::default();
		flat.foot_roll = 0.0;
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		rolled.apply_for(&mut a, 0.5);
		flat.apply_for(&mut b, 0.5);
		let rolled_foot = shin_tail(&a, "shin.L");
		let flat_foot = shin_tail(&b, "shin.L");
		assert!(
			rolled_foot.y > flat_foot.y + 0.03,
			"trailing run pitch should lift the tucked shin (heel-off), rolled={rolled_foot:?} flat={flat_foot:?}"
		);
		Ok(())
	}

	#[test]
	fn run_sample_does_not_accumulate() -> anyhow::Result<()> {
		let mut twice = HumanoidV0Rig::for_clip_test();
		Run::default().apply_for(&mut twice, 0.37);
		Run::default().apply_for(&mut twice, 0.37);
		let once = run_at(0.37);
		for name in once.animation_bone_names() {
			assert!(
				twice.rotation(name).dot(once.rotation(name)).abs() > 1.0 - 1e-5,
				"second sample drifted on {name}"
			);
		}
		Ok(())
	}

	#[test]
	fn run_is_continuous_at_mailbox_frame_intervals() -> anyhow::Result<()> {
		const DT: f32 = 1.0 / 60.0;
		const RUN_CYCLE_SPEED: f32 = 1.68;
		let step = DT * RUN_CYCLE_SPEED;
		// Shin keeps the pre-existing stance/swing knee join; depth channels must stay C0.
		let bones = ["root", "lumbar", "pelvis.L", "femur.L", "shoulder.L", "upper_neck"];
		let mut prev = run_at(0.0);
		let mut progress = 0.0;
		for _ in 0..70 {
			progress += step;
			let now = run_at(progress);
			for name in bones {
				let delta = now.rotation(name).angle_between(prev.rotation(name));
				assert!(delta < 0.22, "jerk on {name} at progress {progress}: delta={delta}");
			}
			prev = now;
		}
		Ok(())
	}

	#[test]
	fn run_loop_seam_matches_the_cycle_wrap() -> anyhow::Result<()> {
		const DT: f32 = 1.0 / 60.0;
		const RUN_CYCLE_SPEED: f32 = 1.68;
		let step = DT * RUN_CYCLE_SPEED;
		let before = run_at(1.0 - step);
		let at_one = run_at(1.0);
		let at_zero = run_at(0.0);
		let after = run_at(step);
		for name in ["femur.L", "pelvis.L", "upper_back", "upper_neck", "root"] {
			assert!(
				at_one.rotation(name).dot(at_zero.rotation(name)).abs() > 1.0 - 1e-5,
				"progress 1 and 0 must match on {name}"
			);
			let seam = at_zero.rotation(name).angle_between(before.rotation(name));
			let interior = after.rotation(name).angle_between(at_zero.rotation(name));
			assert!(
				(seam - interior).abs() < 0.06,
				"seam step on {name} should match a frame step, seam={seam} interior={interior}"
			);
		}
		Ok(())
	}
}
