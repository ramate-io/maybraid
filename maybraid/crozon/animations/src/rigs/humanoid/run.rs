use std::f32::consts::PI;

use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::{animations::Run, Animation};

impl<R: HumanoidRig> Animation<R> for Run<R> {
	fn apply(&self, rig: &mut R) {
		let phase = self.phase.fract();
		let left_arm_swing = -arm_swing(phase + 0.5);
		let right_arm_swing = arm_swing(phase);

		apply_leg(rig, Side::Left, phase + 0.5, -1.0, self);
		apply_leg(rig, Side::Right, phase, 1.0, self);
		apply_arm(
			rig,
			Side::Left,
			left_arm_swing,
			phase,
			rig.forearm_flex_sign(Side::Left),
			-self.arm_down,
			self,
		);
		apply_arm(
			rig,
			Side::Right,
			right_arm_swing,
			phase,
			rig.forearm_flex_sign(Side::Right),
			self.arm_down,
			self,
		);
	}
}

fn apply_leg<R: HumanoidRig>(rig: &mut R, side: Side, phase: f32, lift_sign: f32, run: &Run<R>) {
	let mut leg = rig.leg_pose(side);
	let swing = thigh_swing(phase);

	leg.pelvis = rig.articulate_on_rig(
		leg.pelvis,
		swing * run.hip_swing,
		hip_lift(swing, run.hip_lift) * lift_sign,
	);
	leg.femur = rig.articulate_on_rig(leg.femur, swing * run.stride, 0.0);
	leg.shin = rig.articulate_on_rig(leg.shin, 0.0, knee_flex(phase, run));
	rig.pose_leg(leg);
}

fn apply_arm<R: HumanoidRig>(
	rig: &mut R,
	side: Side,
	arm_swing_value: f32,
	phase: f32,
	flex_sign: f32,
	arm_down: f32,
	run: &Run<R>,
) {
	let mut arm = rig.arm_pose(side);

	arm.shoulder = rig.articulate_on_rig(
		arm.shoulder,
		arm_swing_value * run.shoulder_swing,
		-shoulder_lift(arm_swing_value, run.shoulder_lift),
	);
	arm.humerus =
		rig.articulate_on_rig(arm.humerus, arm_swing_value * run.humerus_swing_scale, arm_down);
	arm.forearm =
		rig.articulate_on_rig(arm.forearm, 0.0, elbow_flex(arm_swing_value, phase, flex_sign, run));
	rig.pose_arm(arm);
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
	let cycle = ((phase + arm_swing.signum() * 0.125) * PI * 4.0).sin().abs();
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
	run.knee_neutral + (t * PI).sin() * (peak - run.knee_neutral)
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;

	#[test]
	fn run_writes_swing_flex_for_left_femur() {
		let mut rig = HumanoidV0Rig::imported();
		Run::<HumanoidV0Rig>::new(0.0).apply(&mut rig);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur pose");
		assert!(femur.swing.abs() > 0.0);
	}

	#[test]
	fn run_left_and_right_femur_swing_are_opposite_at_phase_zero() {
		let mut rig = HumanoidV0Rig::imported();
		Run::<HumanoidV0Rig>::new(0.0).apply(&mut rig);

		let left = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("left femur");
		let right = rig.pose().get(&rig.leg(Side::Right).femur.name).expect("right femur");
		assert!(left.swing * right.swing < 0.0, "legs should be anti-phase");
	}

	#[test]
	fn run_applies_knee_flex_to_shin() {
		let mut rig = HumanoidV0Rig::imported();
		Run::<HumanoidV0Rig>::new(0.25).apply(&mut rig);

		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("left shin");
		assert!(shin.flex.abs() > 0.0, "expected knee bend on shin");
	}

	#[test]
	fn run_applies_elbow_bend_to_forearm() {
		let mut rig = HumanoidV0Rig::imported();
		Run::<HumanoidV0Rig>::new(0.0).apply(&mut rig);

		let left_forearm = rig.pose().get(&rig.arm(Side::Left).forearm.name).expect("left forearm");
		assert!(left_forearm.flex.abs() > 1.0, "expected elbow bend baseline");
	}

	#[test]
	fn run_applies_shoulder_and_hip_lift() {
		let mut rig = HumanoidV0Rig::imported();
		Run::<HumanoidV0Rig>::new(0.0).apply(&mut rig);

		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		let pelvis = rig.pose().get(&rig.leg(Side::Left).pelvis.name).expect("pelvis");
		assert!(shoulder.flex.abs() > 0.0, "expected shoulder bounce");
		assert!(pelvis.flex.abs() > 0.0, "expected hip bounce");
	}
}
