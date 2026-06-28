use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::{UprightWalk, Walk};
use crate::rigs::humanoid::apply::{apply_arm, apply_root};
use crate::{Animation, Effects, Progress};

impl<R: HumanoidRig> Animation<R> for Walk {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		UprightWalk::from_walk(self).apply(rig, progress)
	}
}

impl<R: HumanoidRig> Animation<R> for UprightWalk<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		let phase = Progress(progress).cycle();
		let left_arm_swing = -arm_swing(phase);
		let right_arm_swing = arm_swing(phase + 0.5);
		let walk = self;

		apply_root(rig, walk.torso_lean);
		apply_leg(rig, Side::Left, phase, -1.0, walk);
		apply_leg(rig, Side::Right, phase, 1.0, walk);
		apply_walk_arm(
			rig,
			Side::Left,
			left_arm_swing,
			phase,
			rig.forearm_flex_sign(Side::Left),
			-walk.arm_down,
			walk,
		);
		apply_walk_arm(
			rig,
			Side::Right,
			right_arm_swing,
			phase,
			rig.forearm_flex_sign(Side::Right),
			walk.arm_down,
			walk,
		);

		Effects::default()
	}
}

fn apply_leg<R: HumanoidRig>(
	rig: &mut R,
	side: Side,
	phase: f32,
	lift_sign: f32,
	walk: &UprightWalk<R>,
) {
	let mut leg = rig.leg_pose(side);
	let phase = if side == Side::Left { phase } else { phase + 0.5 };
	let swing = thigh_swing(phase);
	let hip_sagittal = swing * walk.hip_swing * lift_sign;
	let femur_medial = -swing * walk.femur_medial_counter * lift_sign;

	leg.pelvis = rig.articulate_on_rig(
		leg.pelvis,
		hip_sagittal,
		hip_lift(swing, walk.hip_lift) * lift_sign,
	);
	leg.femur = rig.articulate_on_rig(leg.femur, swing * walk.stride, femur_medial);
	leg.shin = rig.articulate_on_rig(leg.shin, 0.0, knee_flex(phase, walk));
	rig.pose_leg(leg);
}

fn apply_walk_arm<R: HumanoidRig>(
	rig: &mut R,
	side: Side,
	arm_swing_value: f32,
	phase: f32,
	flex_sign: f32,
	humerus_flex: f32,
	walk: &UprightWalk<R>,
) {
	apply_arm(
		rig,
		side,
		arm_swing_value * walk.shoulder_swing,
		-shoulder_lift(arm_swing_value, walk.shoulder_lift),
		arm_swing_value * walk.humerus_swing_scale,
		humerus_flex,
		elbow_flex(arm_swing_value, phase, -flex_sign, walk),
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

fn elbow_flex<Rig>(arm_swing: f32, phase: f32, flex_sign: f32, walk: &UprightWalk<Rig>) -> f32 {
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
fn knee_flex<Rig>(leg_phase: f32, walk: &UprightWalk<Rig>) -> f32 {
	let p = leg_phase.fract();
	let t = ((p - 0.5).max(0.0) * 2.0) * std::f32::consts::PI;
	walk.knee_stance_bend + t.sin() * (walk.knee_swing_bend - walk.knee_stance_bend)
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::Run;

	fn assert_pose_matches_at_phases(phases: &[f32]) {
		for &phase in phases {
			let mut from_walk = HumanoidV0Rig::imported();
			let mut from_upright = HumanoidV0Rig::imported();
			Walk::default().apply(&mut from_walk, phase);
			UprightWalk::<HumanoidV0Rig>::default().apply(&mut from_upright, phase);

			for bone in from_walk.animation_bones() {
				let Some(walk_pose) = from_walk.pose().get(&bone) else {
					continue;
				};
				let upright_pose = from_upright.pose().get(&bone).expect("upright pose");
				assert_eq!(walk_pose.swing, upright_pose.swing, "swing mismatch on {bone} at {phase}");
				assert_eq!(walk_pose.flex, upright_pose.flex, "flex mismatch on {bone} at {phase}");
			}
		}
	}

	#[test]
	fn walk_delegates_to_upright_default() {
		assert_pose_matches_at_phases(&[0.0, 0.25, 0.5, 0.75]);
	}

	#[test]
	fn walk_animates_femur_swing() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::<HumanoidV0Rig>::default().apply(&mut rig, 0.0);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur pose");
		assert!(femur.swing.abs() > 0.0);
	}

	#[test]
	fn walk_legs_are_half_cycle_out_of_phase() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::<HumanoidV0Rig>::default().apply(&mut rig, 0.0);

		let left = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("left femur");
		let right = rig.pose().get(&rig.leg(Side::Right).femur.name).expect("right femur");
		assert_ne!(left.swing, right.swing);
	}

	#[test]
	fn walk_stride_is_smaller_than_run() {
		let mut walk_rig = HumanoidV0Rig::imported();
		let mut run_rig = HumanoidV0Rig::imported();
		Walk::default().apply(&mut walk_rig, 0.0);
		Run::default().apply(&mut run_rig, 0.0);

		let walk_femur = walk_rig.pose().get(&walk_rig.leg(Side::Left).femur.name).expect("walk");
		let run_femur = run_rig.pose().get(&run_rig.leg(Side::Left).femur.name).expect("run");
		assert!(walk_femur.swing.abs() < run_femur.swing.abs());
	}

	#[test]
	fn walk_keeps_knee_closer_to_extended_than_run() {
		use bevy::prelude::*;

		let mut walk_rig = HumanoidV0Rig::imported();
		let mut run_rig = HumanoidV0Rig::imported();
		let shin = walk_rig.leg(Side::Left).shin.name.clone();
		for rig in [&mut walk_rig, &mut run_rig] {
			rig.pose_mut()
				.insert(crozon_rigs::BonePose::new(shin.clone(), Transform::IDENTITY));
		}

		Walk::default().apply(&mut walk_rig, 0.75);
		Run::default().apply(&mut run_rig, 0.75);

		let walk_shin = walk_rig.pose().get(&shin).expect("walk shin");
		let run_shin = run_rig.pose().get(&shin).expect("run shin");
		assert!(walk_shin.flex < run_shin.flex);
	}

	#[test]
	fn walk_applies_forward_torso_lean() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::<HumanoidV0Rig>::default().apply(&mut rig, 0.0);

		let root = rig.pose().get(&rig.spine().root.name).expect("root");
		assert!(root.swing > 0.05);
	}

	#[test]
	fn walk_femur_counters_hip_swing_out() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::<HumanoidV0Rig>::default().apply(&mut rig, 0.0);

		let pelvis = rig.pose().get(&rig.leg(Side::Left).pelvis.name).expect("pelvis");
		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		assert!(pelvis.swing.abs() > 0.0);
		assert!(femur.flex.signum() != pelvis.swing.signum());
		assert!(femur.flex.abs() > 0.0);
	}

	#[test]
	fn walk_stance_leg_has_soft_knee_bend() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::<HumanoidV0Rig>::default().apply(&mut rig, 0.0);

		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("shin");
		assert!(shin.flex > 0.0);
	}

	#[test]
	fn walk_knee_flex_is_continuous_across_stride() {
		let walk = UprightWalk::<HumanoidV0Rig>::default();
		let samples = 120;
		let max_step = (walk.knee_swing_bend - walk.knee_stance_bend) * 2.0 * std::f32::consts::PI
			/ samples as f32
			+ 1e-4;
		let mut prev = knee_flex(0.0, &walk);
		for i in 1..=samples {
			let phase = i as f32 / samples as f32;
			let flex = knee_flex(phase, &walk);
			assert!(
				(flex - prev).abs() < max_step,
				"knee snap at phase {phase}: {prev} -> {flex}"
			);
			prev = flex;
		}
	}

	#[test]
	fn walk_vertical_bob_comes_mostly_from_hips() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::<HumanoidV0Rig>::default().apply(&mut rig, 0.0);

		let pelvis = rig.pose().get(&rig.leg(Side::Left).pelvis.name).expect("pelvis");
		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		assert!(pelvis.flex.abs() > shoulder.flex.abs());
	}
}
