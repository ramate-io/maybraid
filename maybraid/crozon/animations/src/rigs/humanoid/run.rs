use std::f32::consts::PI;

use crozon_rigs::{humanoid::HumanoidRig, BonePose, Side};

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
	let mut leg = rig.leg(side);
	let swing = thigh_swing(phase);

	leg.pelvis = BonePose::with_articulation(
		leg.pelvis.name,
		swing * run.hip_swing,
		hip_lift(swing, run.hip_lift) * lift_sign,
	);
	leg.femur = BonePose::with_articulation(leg.femur.name, swing * run.stride, 0.0);
	leg.shin = BonePose::with_articulation(leg.shin.name, 0.0, knee_flex(phase, run));
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
	let mut arm = rig.arm(side);

	arm.shoulder = BonePose::with_articulation(
		arm.shoulder.name,
		arm_swing_value * run.shoulder_swing,
		-shoulder_lift(arm_swing_value, run.shoulder_lift),
	);
	arm.humerus = BonePose::with_articulation(
		arm.humerus.name,
		arm_swing_value * run.humerus_swing_scale,
		arm_down,
	);
	arm.forearm = BonePose::with_articulation(
		arm.forearm.name,
		0.0,
		elbow_flex(arm_swing_value, phase, flex_sign, run),
	);
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
	use crozon_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;

	#[test]
	fn run_writes_swing_flex_for_left_femur() {
		let mut rig = HumanoidV0Rig::imported();
		Run::<HumanoidV0Rig>::new(0.0).apply(&mut rig);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur pose");
		assert!(femur.swing.abs() > 0.0);
	}
}
