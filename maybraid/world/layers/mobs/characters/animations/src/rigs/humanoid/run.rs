use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::animations::{Run, UprightRun};
use crate::rigs::humanoid::gait_coordination::GaitCoordination;
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
		let gait = GaitCoordination::from_progress(progress);
		let phase = gait.phase();
		let left_arm_swing = -GaitCoordination::arm_swing(phase);
		let right_arm_swing = GaitCoordination::arm_swing(phase + 0.5);

		gait.apply_spine_counter(&mut pose, self.torso_lean, self.torso_turn, self.side_lean);
		apply_leg(&mut pose, Side::Left, phase, -1.0, gait, self);
		apply_leg(&mut pose, Side::Right, phase, 1.0, gait, self);
		// Both elbows share one flexion sign. Opposite arm_down values are a hang bias.
		apply_run_arm(&mut pose, Side::Left, left_arm_swing, phase, -self.arm_down, gait, self);
		apply_run_arm(&mut pose, Side::Right, right_arm_swing, phase, self.arm_down, gait, self);
		gait.apply_head(
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
	cycle: f32,
	lift_sign: f32,
	gait: GaitCoordination,
	run: &UprightRun,
) {
	let phase = if side == Side::Left { cycle } else { cycle + 0.5 };
	let swing = GaitCoordination::thigh_swing(phase);
	gait.apply_pelvis_yaw_and_bob(
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
	phase: f32,
	humerus_flex: f32,
	gait: GaitCoordination,
	run: &UprightRun,
) {
	gait.apply_arm_across(
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
			phase,
			-1.0,
			run.elbow_bend,
			run.elbow_pump,
			run.elbow_cycle,
		),
	);
}

fn knee_flex(leg_phase: f32, run: &UprightRun) -> f32 {
	let p = leg_phase.fract();
	if p < 0.25 {
		ease_between(run.knee_neutral, run.knee_extended, p / 0.25)
	} else if p < 0.5 {
		ease_between(run.knee_extended, run.knee_neutral, (p - 0.25) / 0.25)
	} else if p < 0.60 {
		ease_between(run.knee_neutral, run.knee_contracted, (p - 0.5) / 0.10)
	} else if p < 0.85 {
		ease_between(run.knee_contracted, run.knee_extended, (p - 0.60) / 0.25)
	} else {
		ease_between(run.knee_extended, run.knee_neutral, (p - 0.85) / 0.15)
	}
}

fn ease_between(from: f32, to: f32, progress: f32) -> f32 {
	from + (to - from) * smoothstep(progress)
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
	fn run_knees_keep_authored_extrema_and_are_cyclically_smooth() -> anyhow::Result<()> {
		let mut extended = HumanoidV0Rig::for_clip_test();
		let mut contracted = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut extended, 0.25);
		Run::default().apply(&mut contracted, 0.60);
		if extended.posed_angle("shin.L") > 0.05 || contracted.posed_angle("shin.L") <= 0.5 {
			anyhow::bail!("run should preserve its straight and contracted knee landmarks");
		}

		let epsilon = 1.0 / 240.0;
		for boundary in [0.5, 1.0] {
			let before = run_at(boundary - epsilon);
			let center = run_at(boundary);
			let after = run_at(boundary + epsilon);
			for name in ["shin.L", "shin.R"] {
				let before_tip = shin_tip(&before, name);
				let center_tip = shin_tip(&center, name);
				let after_tip = shin_tip(&after, name);
				if before_tip.distance(center_tip) >= 0.03 || center_tip.distance(after_tip) >= 0.03
				{
					anyhow::bail!(
						"run knee FK must stay continuous around phase {boundary} on {name}: {before_tip:?} {center_tip:?} {after_tip:?}"
					);
				}
			}
		}
		Ok(())
	}

	#[test]
	fn run_foot_steps_refine_near_inverse_sample_rate() -> anyhow::Result<()> {
		let step_60 = max_foot_step(60);
		let step_120 = max_foot_step(120);
		let step_240 = max_foot_step(240);
		let step_480 = max_foot_step(480);
		let ratios = [step_120 / step_60, step_240 / step_120, step_480 / step_240];
		if step_60 <= 0.01 || ratios.iter().any(|ratio| !(0.35..0.70).contains(ratio)) {
			anyhow::bail!(
				"run FK step should refine near 1/rate: 60={step_60:.5}, 120={step_120:.5}, 240={step_240:.5}, 480={step_480:.5}, ratios={ratios:?}"
			);
		}
		Ok(())
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

	fn shin_tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.character_point(name) + rig.character_length(name) * rig.segment_lengths.shin
	}

	fn max_foot_step(samples_per_cycle: u32) -> f32 {
		let mut previous = run_at(0.0);
		let mut largest = 0.0_f32;
		for frame in 1..=samples_per_cycle {
			let progress = frame as f32 / samples_per_cycle as f32;
			let current = run_at(progress);
			for name in ["shin.L", "shin.R"] {
				largest = largest.max(shin_tip(&previous, name).distance(shin_tip(&current, name)));
			}
			previous = current;
		}
		largest
	}

	#[test]
	fn run_lean_pelvis_yaw_and_head_counter_are_visible_on_bind() -> anyhow::Result<()> {
		let run = run_at(0.0);
		let mut walk = HumanoidV0Rig::for_clip_test();
		crate::animations::Walk::default().apply_for(&mut walk, 0.0);
		let root = run.rotation("root") * Vec3::Y;
		let walk_root = walk.rotation("root") * Vec3::Y;
		assert!(root.z > 0.14 && root.x.abs() < 1e-3, "run lean is forward, {root:?}");
		assert!(root.z > walk_root.z + 0.04, "run leans farther than walk");
		let left_hip = run.character_point("femur.L");
		let right_hip = run.character_point("femur.R");
		assert!(left_hip.z - right_hip.z > 0.05, "pelvis yaw splits hips in Z");
		let chest = run.rotation("root") * Vec3::Y;
		let head = run.character_length("upper_neck");
		assert!(head.z.abs() < chest.z - 0.03, "head counters chest lean: {head:?}, {chest:?}");
		Ok(())
	}

	#[test]
	fn run_midstance_bob_and_trailing_foot_roll_change_composed_geometry() -> anyhow::Result<()> {
		let mid = run_at(0.0);
		let passing = run_at(0.25);
		let mean_hip_y = |rig: &HumanoidV0Rig| {
			0.5 * (rig.character_point("femur.L").y + rig.character_point("femur.R").y)
		};
		assert!(mean_hip_y(&mid) > mean_hip_y(&passing) + 0.015);

		let mut flat = UprightRun::default();
		flat.foot_roll = 0.0;
		let mut rolled_rig = HumanoidV0Rig::for_clip_test();
		let mut flat_rig = HumanoidV0Rig::for_clip_test();
		UprightRun::default().apply_for(&mut rolled_rig, 0.5);
		flat.apply_for(&mut flat_rig, 0.5);
		let rolled = shin_tip(&rolled_rig, "shin.L");
		let unrolled = shin_tip(&flat_rig, "shin.L");
		assert!(
			rolled.y > unrolled.y + 0.03,
			"trailing shin should lift: {rolled:?} vs {unrolled:?}"
		);
		Ok(())
	}

	#[test]
	fn run_depth_channels_remain_continuous_at_mailbox_rate() -> anyhow::Result<()> {
		let bones = ["root", "lumbar", "pelvis.L", "femur.L", "shoulder.L", "upper_neck"];
		let mut previous = run_at(0.0);
		for frame in 1..=70 {
			let progress = frame as f32 * 1.68 / 60.0;
			let current = run_at(progress);
			for name in bones {
				let delta = current.rotation(name).angle_between(previous.rotation(name));
				assert!(delta < 0.22, "60 Hz run jump on {name} at {progress}: {delta}");
			}
			previous = current;
		}
		Ok(())
	}
}
