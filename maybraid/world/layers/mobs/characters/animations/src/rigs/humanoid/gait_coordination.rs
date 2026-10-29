//! Shared walk/run pelvis, torso counter-turn, arm-across, and head stabilization.
//!
//! Walk and run supply their own amplitudes and keep stride, knee, and contact curves separate.

use std::f32::consts::{PI, TAU};

use character_rigs::authoring::HumanoidPose;
use character_rigs::Side;

use crate::Progress;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaitCoordination {
	phase: f32,
}

impl GaitCoordination {
	pub fn from_progress(progress: f32) -> Self {
		Self { phase: Progress(progress).cycle() }
	}

	pub fn phase(self) -> f32 {
		self.phase
	}

	pub fn thigh_swing(phase: f32) -> f32 {
		let p = phase.rem_euclid(1.0);
		if p < 0.5 {
			4.0 * p - 1.0
		} else {
			3.0 - 4.0 * p
		}
	}

	pub fn gait(self) -> f32 {
		Self::thigh_swing(self.phase)
	}

	pub fn arm_swing(phase: f32) -> f32 {
		Self::thigh_swing(phase) * 0.75
	}

	pub fn body_bob(self) -> f32 {
		(self.phase * TAU * 2.0).cos()
	}

	pub fn apply_spine_counter(
		self,
		pose: &mut HumanoidPose,
		torso_lean: f32,
		torso_turn: f32,
		side_lean: f32,
	) {
		let gait = self.gait();
		pose.apply_root(torso_lean);
		pose.spine.add_turn(gait * torso_turn);
		pose.spine.side_bend[1] += gait * side_lean * 0.6;
		pose.spine.side_bend[2] += gait * side_lean * 0.4;
	}

	pub fn apply_pelvis_yaw_and_bob(
		self,
		pose: &mut HumanoidPose,
		side: Side,
		swing: f32,
		lift_sign: f32,
		hip_swing: f32,
		hip_lift: f32,
		body_bob: f32,
	) {
		let leg = pose.leg_mut(side);
		leg.pelvis_turn = self.gait() * hip_swing;
		let hike = swing * hip_lift * lift_sign;
		let vertical = self.body_bob() * body_bob * side.sign();
		leg.pelvis_lateral = hike + vertical;
	}

	pub fn apply_head(
		self,
		pose: &mut HumanoidPose,
		torso_lean: f32,
		torso_turn: f32,
		side_lean: f32,
		stabilize: f32,
	) {
		let gait = self.gait();
		let nod = -torso_lean * 0.55 * stabilize;
		let turn = -gait * torso_turn * 0.65 * stabilize;
		let tilt = -gait * side_lean * 0.7 * stabilize;
		pose.apply_neck_twisted(
			turn * 0.6,
			tilt * 0.5,
			nod * 0.4,
			turn * 0.4,
			tilt * 0.5,
			nod * 0.6,
		);
	}

	pub fn apply_arm_across(
		self,
		pose: &mut HumanoidPose,
		side: Side,
		arm_swing_value: f32,
		humerus_flex: f32,
		shoulder_swing: f32,
		shoulder_lift: f32,
		humerus_swing_scale: f32,
		arm_across: f32,
		elbow_flex: f32,
	) {
		let forward = (-side.sign() * arm_swing_value).max(0.0);
		let across = -side.sign() * forward * arm_across;
		pose.apply_arm_twisted(
			side,
			arm_swing_value * shoulder_swing,
			-(arm_swing_value * shoulder_lift) + across,
			across * 0.35,
			arm_swing_value * humerus_swing_scale,
			humerus_flex + across * 0.45,
			elbow_flex,
		);
	}

	pub fn elbow_flex(
		arm_swing: f32,
		phase: f32,
		flex_sign: f32,
		elbow_bend: f32,
		elbow_pump: f32,
		elbow_cycle: f32,
	) -> f32 {
		let pump = arm_swing.abs();
		let cycle = ((phase + arm_swing.signum() * 0.125) * PI * 4.0).sin().abs();
		flex_sign * (elbow_bend + pump * elbow_pump + cycle * elbow_cycle)
	}
}
