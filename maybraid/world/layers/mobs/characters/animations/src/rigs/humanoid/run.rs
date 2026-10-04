use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Run, UprightRun};
use crate::rigs::humanoid::apply::apply_arm;
use crate::{Animation, Progress};

impl Animation<HumanoidV0Rig> for Run {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		UprightRun::from_run(self).apply_for(rig, progress)
	}
}

impl Animation<HumanoidV0Rig> for UprightRun {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let phase = Progress(progress).cycle();
		let left_arm_swing = -arm_swing(phase);
		let right_arm_swing = arm_swing(phase + 0.5);

		apply_leg(&mut pose, Side::Left, phase, -1.0, self);
		apply_leg(&mut pose, Side::Right, phase, 1.0, self);
		// Both elbows share one flexion sign. Opposite arm_down values are a hang bias.
		apply_run_arm(&mut pose, Side::Left, left_arm_swing, phase, -self.arm_down, self);
		apply_run_arm(&mut pose, Side::Right, right_arm_swing, phase, self.arm_down, self);
		rig.write_pose(&pose);
	}
}

fn apply_leg(pose: &mut HumanoidPose, side: Side, phase: f32, lift_sign: f32, run: &UprightRun) {
	let phase = if side == Side::Left { phase } else { phase + 0.5 };
	let swing = thigh_swing(phase);
	let leg = pose.leg_mut(side);
	// Pelvis yaw stays axial. Hip lift is a lateral hike; lift_sign is gait bias.
	leg.pelvis_turn = swing * run.hip_swing;
	leg.pelvis_lateral = hip_lift(swing, run.hip_lift) * lift_sign;
	leg.hip_flexion = swing * run.stride;
	leg.knee_flexion = knee_flex(phase, run) - run.knee_extended;
}

fn apply_run_arm(
	pose: &mut HumanoidPose,
	side: Side,
	arm_swing_value: f32,
	phase: f32,
	humerus_flex: f32,
	run: &UprightRun,
) {
	apply_arm(
		pose,
		side,
		arm_swing_value * run.shoulder_swing,
		-shoulder_lift(arm_swing_value, run.shoulder_lift),
		arm_swing_value * run.humerus_swing_scale,
		humerus_flex,
		elbow_flex(arm_swing_value, phase, -1.0, run),
	);
}

fn thigh_swing(phase: f32) -> f32 {
	let p = phase.fract();
	if p < 0.5 {
		4.0 * p - 1.0
	} else {
		3.0 - 4.0 * p
	}
}

fn arm_swing(phase: f32) -> f32 {
	thigh_swing(phase) * 0.75
}

fn elbow_flex(arm_swing: f32, phase: f32, flex_sign: f32, run: &UprightRun) -> f32 {
	let pump = arm_swing.abs();
	let cycle = ((phase + arm_swing.signum() * 0.125) * std::f32::consts::PI * 4.0).sin().abs();
	flex_sign * (run.elbow_bend + pump * run.elbow_pump + cycle * run.elbow_cycle)
}

fn shoulder_lift(arm_swing: f32, amplitude: f32) -> f32 {
	arm_swing * amplitude
}

fn hip_lift(leg_swing: f32, amplitude: f32) -> f32 {
	leg_swing * amplitude
}

fn knee_flex(leg_phase: f32, run: &UprightRun) -> f32 {
	let p = leg_phase.fract();
	let peak = if p < 0.5 { run.knee_extended } else { run.knee_contracted };
	let t = if p < 0.5 { p * 2.0 } else { (p - 0.5) * 2.0 };
	run.knee_neutral + (t * std::f32::consts::PI).sin() * (peak - run.knee_neutral)
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

		let femur = tip(&rig, "femur.L");
		assert!(femur.z.abs() > 0.2, "stride bends forward/back, got {femur:?}");
		assert!(femur.x.abs() < femur.z.abs(), "stride stays mostly sagittal, got {femur:?}");
	}

	#[test]
	fn run_right_leg_uses_half_cycle_phase_offset() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.0);

		let left = tip(&rig, "femur.L");
		let right = tip(&rig, "femur.R");
		assert!((left - right).length() > 0.05, "left {left:?} right {right:?}");
	}

	#[test]
	fn run_applies_knee_flex_to_shin() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.25);
		let extended = tip(&rig, "shin.L");
		let extended_rot = rig.rotation("shin.L");
		assert!((extended - Vec3::Y).length() < 1e-3, "expected straight knee, got {extended:?}");

		UprightRun::default().apply(&mut rig, 0.75);
		let tucked = tip(&rig, "shin.L");
		assert!(tucked.z > 0.5, "expected knee tuck toward +Z, got {tucked:?}");
		assert!(tucked.y < 0.6, "tuck passes one radian, got {tucked:?}");
		assert!(
			extended_rot.dot(rig.rotation("shin.L")).abs() < 0.95,
			"knee rotation should change across stride"
		);
	}

	#[test]
	fn run_applies_elbow_bend_to_forearm() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.0);

		let forearm = tip(&rig, "forearm.L");
		assert!(
			(forearm - Vec3::Y).length() > 0.9,
			"expected elbow bend baseline, got {forearm:?}"
		);
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
}
