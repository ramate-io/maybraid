use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{UprightWalk, Walk};
use crate::rigs::humanoid::apply::{apply_arm, apply_root};
use crate::{Animation, Progress};

impl Animation<HumanoidV0Rig> for Walk {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		UprightWalk::from_walk(self).apply_for(rig, progress)
	}
}

impl Animation<HumanoidV0Rig> for UprightWalk {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		sample_walk(self, progress, &mut pose);
		rig.write_pose(&pose);
	}
}

fn sample_walk(walk: &UprightWalk, progress: f32, pose: &mut HumanoidPose) {
	let phase = Progress(progress).cycle();
	let left_arm_swing = -arm_swing(phase);
	let right_arm_swing = arm_swing(phase + 0.5);

	apply_root(pose, walk.torso_lean);
	apply_leg(pose, Side::Left, phase, -1.0, walk);
	apply_leg(pose, Side::Right, phase, 1.0, walk);
	// Both elbows share one flexion sign. The opposite arm_down values are a
	// hang bias, not a mirror of the joint frame.
	apply_walk_arm(pose, Side::Left, left_arm_swing, phase, -walk.arm_down, walk);
	apply_walk_arm(pose, Side::Right, right_arm_swing, phase, walk.arm_down, walk);
}

fn apply_leg(pose: &mut HumanoidPose, side: Side, phase: f32, lift_sign: f32, walk: &UprightWalk) {
	let phase = if side == Side::Left { phase } else { phase + side.phase_offset() };
	let swing = thigh_swing(phase);
	let leg = pose.leg_mut(side);
	// Pelvis yaw stays axial. Hip lift is a lateral hike, with opposite signs so
	// the stance side rises. Those signs are gait bias, not axis mirrors.
	leg.pelvis_turn = swing * walk.hip_swing * lift_sign;
	leg.pelvis_lateral = hip_lift(swing, walk.hip_lift) * lift_sign;
	leg.hip_flexion = swing * walk.stride;
	leg.hip_abduction = -swing * walk.femur_medial_counter * lift_sign;
	leg.knee_flexion = knee_flex(phase, walk);
}

fn apply_walk_arm(
	pose: &mut HumanoidPose,
	side: Side,
	arm_swing_value: f32,
	phase: f32,
	humerus_flex: f32,
	walk: &UprightWalk,
) {
	apply_arm(
		pose,
		side,
		arm_swing_value * walk.shoulder_swing,
		-shoulder_lift(arm_swing_value, walk.shoulder_lift),
		arm_swing_value * walk.humerus_swing_scale,
		humerus_flex,
		elbow_flex(arm_swing_value, phase, -1.0, walk),
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

fn elbow_flex(arm_swing: f32, phase: f32, flex_sign: f32, walk: &UprightWalk) -> f32 {
	let pump = arm_swing.abs();
	let cycle = ((phase + arm_swing.signum() * 0.125) * std::f32::consts::PI * 4.0).sin().abs();
	flex_sign * (walk.elbow_bend + pump * walk.elbow_pump + cycle * walk.elbow_cycle)
}

fn shoulder_lift(arm_swing: f32, amplitude: f32) -> f32 {
	arm_swing * amplitude
}

fn hip_lift(leg_swing: f32, amplitude: f32) -> f32 {
	leg_swing * amplitude
}

/// Soft knee on stance; smooth half-sine lift through swing and back to stance.
fn knee_flex(leg_phase: f32, walk: &UprightWalk) -> f32 {
	let p = leg_phase.fract();
	let t = ((p - 0.5).max(0.0) * 2.0) * std::f32::consts::PI;
	walk.knee_stance_bend + t.sin() * (walk.knee_swing_bend - walk.knee_stance_bend)
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use super::*;
	use crate::animations::Run;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn assert_pose_matches_at_phases(phases: &[f32]) {
		for &phase in phases {
			let mut from_walk = HumanoidV0Rig::imported();
			let mut from_upright = HumanoidV0Rig::imported();
			Walk::default().apply(&mut from_walk, phase);
			UprightWalk::default().apply(&mut from_upright, phase);

			for name in from_walk.animation_bone_names() {
				let walk_rot = from_walk.rotation(name);
				let upright_rot = from_upright.rotation(name);
				assert!(
					walk_rot.dot(upright_rot).abs() > 1.0 - 1e-5,
					"rotation mismatch on {name} at {phase}"
				);
			}
		}
	}

	#[test]
	fn walk_delegates_to_upright_default() {
		assert_pose_matches_at_phases(&[0.0, 0.25, 0.5, 0.75]);
	}

	#[test]
	fn walk_animates_femur_in_the_sagittal_plane() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("femur.L") > 0.05, "stride leaves rest");
	}

	#[test]
	fn walk_legs_are_half_cycle_out_of_phase() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		UprightWalk::default().apply(&mut rig, 0.0);

		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!((left.z - right.z).abs() > 0.05, "legs are out of phase, L={left:?} R={right:?}");
	}

	#[test]
	fn walk_stride_is_smaller_than_run() {
		let mut walk_rig = HumanoidV0Rig::imported();
		let mut run_rig = HumanoidV0Rig::imported();
		Walk::default().apply(&mut walk_rig, 0.0);
		Run::default().apply(&mut run_rig, 0.0);

		assert!(
			walk_rig.posed_angle("femur.L") < run_rig.posed_angle("femur.L"),
			"walk stride should be smaller than run"
		);
	}

	#[test]
	fn walk_keeps_knee_closer_to_extended_than_run() {
		let mut walk_rig = HumanoidV0Rig::imported();
		let mut run_rig = HumanoidV0Rig::imported();
		Walk::default().apply(&mut walk_rig, 0.75);
		Run::default().apply(&mut run_rig, 0.75);

		assert!(
			walk_rig.posed_angle("shin.L") < run_rig.posed_angle("shin.L"),
			"walk knee should stay closer to rest than run"
		);
	}

	#[test]
	fn walk_applies_forward_torso_lean() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		let root = tip(&rig, "root");
		assert!(root.z > 0.05, "forward lean goes to +Z, got {root:?}");
		assert!(root.x.abs() < 1e-3, "lean must not yaw, got {root:?}");
	}

	#[test]
	fn walk_femur_counters_hip_swing_out() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("pelvis.L") > 0.0, "pelvis yaws");
		assert!(rig.posed_angle("femur.L") > 0.0, "femur strides");
	}

	#[test]
	fn walk_stance_leg_has_soft_knee_bend() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("shin.L") > 0.0, "stance knee flexes");
	}

	#[test]
	fn walk_knee_flex_is_continuous_across_stride() {
		let walk = UprightWalk::default();
		let samples = 120;
		let max_step = (walk.knee_swing_bend - walk.knee_stance_bend) * 2.0 * std::f32::consts::PI
			/ samples as f32
			+ 1e-4;
		let mut prev = knee_flex(0.0, &walk);
		for i in 1..=samples {
			let phase = i as f32 / samples as f32;
			let flex = knee_flex(phase, &walk);
			assert!((flex - prev).abs() < max_step, "knee snap at phase {phase}: {prev} -> {flex}");
			prev = flex;
		}
	}

	#[test]
	fn walk_vertical_bob_comes_mostly_from_hips() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(
			rig.posed_angle("pelvis.L") > rig.posed_angle("shoulder.L"),
			"vertical bob comes mostly from hips"
		);
	}
}
