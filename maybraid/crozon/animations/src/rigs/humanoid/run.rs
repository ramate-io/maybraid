use bevy::prelude::*;
use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::{animations::Run, Animation};

impl<R: HumanoidRig> Animation<R> for Run<R> {
	fn apply(&self, rig: &mut R) {
		let phase = self.phase.fract();
		apply_leg(rig, Side::Left, phase + 0.5, -1.0, self);
		apply_leg(rig, Side::Right, phase, 1.0, self);
		apply_arm(rig, Side::Left, -arm_swing(phase + 0.5) * self.arm_swing, -self.arm_down, self);
		apply_arm(rig, Side::Right, arm_swing(phase) * self.arm_swing, self.arm_down, self);
	}
}

fn apply_leg<R: HumanoidRig>(
	rig: &mut R,
	side: Side,
	phase: f32,
	hip_lift_sign: f32,
	run: &Run<R>,
) {
	let leg = rig.leg(side);
	let swing = thigh_swing(phase);

	rig.pose_mut().set_transform(
		leg.pelvis,
		Transform::from_rotation(Quat::from_rotation_x(swing * run.hip_swing * hip_lift_sign)),
	);
	rig.pose_mut().set_transform(
		leg.femur,
		Transform::from_rotation(Quat::from_rotation_x(swing * run.stride)),
	);
	rig.pose_mut().set_transform(
		leg.shin,
		Transform::from_rotation(Quat::from_rotation_x(knee_flex(phase, run))),
	);
}

fn apply_arm<R: HumanoidRig>(rig: &mut R, side: Side, swing: f32, arm_down: f32, run: &Run<R>) {
	let arm = rig.arm(side);
	let elbow = run.elbow_bend + swing.abs() * run.elbow_pump;

	rig.pose_mut()
		.set_transform(arm.shoulder, Transform::from_rotation(Quat::from_rotation_y(swing * 0.14)));
	rig.pose_mut().set_transform(
		arm.humerus,
		Transform::from_rotation(Quat::from_rotation_y(swing) * Quat::from_rotation_z(arm_down)),
	);
	rig.pose_mut()
		.set_transform(arm.forearm, Transform::from_rotation(Quat::from_rotation_y(elbow)));
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
	thigh_swing(phase)
}

fn knee_flex<Rig>(phase: f32, run: &Run<Rig>) -> f32 {
	let p = phase.fract();
	let peak = if p < 0.5 { run.knee_extended } else { run.knee_contracted };
	let t = if p < 0.5 { p * 2.0 } else { (p - 0.5) * 2.0 };
	run.knee_neutral + (t * std::f32::consts::PI).sin() * (peak - run.knee_neutral)
}
