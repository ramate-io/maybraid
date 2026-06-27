use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::animations::Run;
use crate::rigs::humanoid::apply::apply_arm;
use crate::{Animation, Effects, Progress};

impl<R: HumanoidRig> Animation<R> for Run<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		let phase = Progress(progress).cycle();
		let left_arm_swing = -arm_swing(phase);
		let right_arm_swing = arm_swing(phase + 0.5);
		let run = self;

		apply_leg(rig, Side::Left, phase, -1.0, run);
		apply_leg(rig, Side::Right, phase, 1.0, run);
		apply_run_arm(
			rig,
			Side::Left,
			left_arm_swing,
			phase,
			rig.forearm_flex_sign(Side::Left),
			-run.arm_down,
			run,
		);
		apply_run_arm(
			rig,
			Side::Right,
			right_arm_swing,
			phase,
			rig.forearm_flex_sign(Side::Right),
			run.arm_down,
			run,
		);

		Effects::default()
	}
}

fn apply_leg<R: HumanoidRig>(rig: &mut R, side: Side, phase: f32, lift_sign: f32, run: &Run<R>) {
	let mut leg = rig.leg_pose(side);
	let phase = if side == Side::Left { phase } else { phase + 0.5 };
	let swing = thigh_swing(phase);

	leg.pelvis = rig.articulate_on_rig(
		leg.pelvis,
		swing * run.hip_swing,
		hip_lift(swing, run.hip_lift) * lift_sign,
	);
	leg.femur = rig.articulate_on_rig(leg.femur, swing * run.stride, 0.0);
	leg.shin = rig.articulate_on_rig(leg.shin, 0.0, knee_flex(phase, run) - run.knee_extended);
	rig.pose_leg(leg);
}

fn apply_run_arm<R: HumanoidRig>(
	rig: &mut R,
	side: Side,
	arm_swing_value: f32,
	phase: f32,
	flex_sign: f32,
	humerus_flex: f32,
	run: &Run<R>,
) {
	apply_arm(
		rig,
		side,
		arm_swing_value * run.shoulder_swing,
		-shoulder_lift(arm_swing_value, run.shoulder_lift),
		arm_swing_value * run.humerus_swing_scale,
		humerus_flex,
		elbow_flex(arm_swing_value, phase, -flex_sign, run),
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

fn elbow_flex<Rig>(arm_swing: f32, phase: f32, flex_sign: f32, run: &Run<Rig>) -> f32 {
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

fn knee_flex<Rig>(leg_phase: f32, run: &Run<Rig>) -> f32 {
	let p = leg_phase.fract();
	let peak = if p < 0.5 { run.knee_extended } else { run.knee_contracted };
	let t = if p < 0.5 { p * 2.0 } else { (p - 0.5) * 2.0 };
	run.knee_neutral + (t * std::f32::consts::PI).sin() * (peak - run.knee_neutral)
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;

	#[test]
	fn run_writes_swing_flex_for_left_femur() {
		let mut rig = HumanoidV0Rig::imported();
		Run::<HumanoidV0Rig>::default().apply(&mut rig, 0.0);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur pose");
		assert!(femur.swing.abs() > 0.0);
	}

	#[test]
	fn run_right_leg_uses_half_cycle_phase_offset() {
		let mut rig = HumanoidV0Rig::imported();
		Run::<HumanoidV0Rig>::default().apply(&mut rig, 0.0);

		let left = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("left femur");
		let right = rig.pose().get(&rig.leg(Side::Right).femur.name).expect("right femur");
		assert_ne!(left.swing, right.swing);
	}

	#[test]
	fn run_applies_knee_flex_to_shin() {
		use bevy::prelude::*;

		let mut rig = HumanoidV0Rig::imported();
		let shin_name = rig.leg(Side::Left).shin.name.clone();
		rig.pose_mut()
			.insert(crozon_rigs::BonePose::new(shin_name.clone(), Transform::IDENTITY));

		Run::<HumanoidV0Rig>::default().apply(&mut rig, 0.25);
		let extended = rig.pose().get(&shin_name).expect("left shin").clone();
		assert!(extended.flex.abs() < 1e-4, "expected straight knee, flex={}", extended.flex);

		Run::<HumanoidV0Rig>::default().apply(&mut rig, 0.75);
		let tucked = rig.pose().get(&shin_name).expect("left shin");
		assert!(tucked.flex > 1.0, "expected knee tuck, flex={}", tucked.flex);
		assert!(
			extended.transform.rotation.dot(tucked.transform.rotation).abs() < 0.95,
			"knee rotation should change across stride"
		);
	}

	#[test]
	fn run_applies_elbow_bend_to_forearm() {
		let mut rig = HumanoidV0Rig::imported();
		Run::<HumanoidV0Rig>::default().apply(&mut rig, 0.0);

		let left_forearm = rig.pose().get(&rig.arm(Side::Left).forearm.name).expect("left forearm");
		assert!(left_forearm.flex.abs() > 1.0, "expected elbow bend baseline");
	}

	#[test]
	fn run_applies_shoulder_and_hip_lift() {
		let mut rig = HumanoidV0Rig::imported();
		Run::<HumanoidV0Rig>::default().apply(&mut rig, 0.0);

		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		let pelvis = rig.pose().get(&rig.leg(Side::Left).pelvis.name).expect("pelvis");
		assert!(shoulder.flex.abs() > 0.0, "expected shoulder bounce");
		assert!(pelvis.flex.abs() > 0.0, "expected hip bounce");
	}
}
