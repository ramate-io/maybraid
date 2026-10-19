//! Shared walk/run pelvis yaw, spine counter-turn, arm-across, and head stabilize.
//!
//! Walk and run author different coefficients into the same coordination structure.
//! This type holds only the cycle oscillator; callers pass clip knobs. Knee, stride,
//! and contact / foot-roll curves stay on each clip so `Mix<Walk, Run>` can keep
//! those envelopes independent ([#1344](https://github.com/ramate-io/maybraid/pull/1344)).

use std::f32::consts::{PI, TAU};

use character_rigs::authoring::HumanoidPose;
use character_rigs::Side;

use crate::Progress;

/// Cycle-local oscillators plus the shared coordination writes.
///
/// Not a universal gait parameter block: stride, knee, and contact stay off this type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaitCoordination {
	phase: f32,
}

impl GaitCoordination {
	/// Wrap `progress` onto one gait cycle in `[0, 1)`.
	pub fn from_progress(progress: f32) -> Self {
		Self { phase: Progress(progress).cycle() }
	}

	/// Cycle phase in `[0, 1)`.
	pub fn phase(self) -> f32 {
		self.phase
	}

	/// Triangle oscillator in `[-1, 1]`, period 1. Shared gait signal for yaw / spine.
	pub fn thigh_swing(phase: f32) -> f32 {
		let p = phase.fract();
		if p < 0.5 {
			4.0 * p - 1.0
		} else {
			3.0 - 4.0 * p
		}
	}

	/// [`Self::thigh_swing`] at this cycle.
	pub fn gait(self) -> f32 {
		Self::thigh_swing(self.phase)
	}

	/// Arm opposite the matching thigh, scaled 0.75.
	pub fn arm_swing(phase: f32) -> f32 {
		Self::thigh_swing(phase) * 0.75
	}

	/// Twice-per-cycle cosine, high at midstance (phase 0 / 0.5).
	pub fn body_bob(self) -> f32 {
		(self.phase * TAU * 2.0).cos()
	}

	/// Root lean plus shoulder-girdle yaw and waist side-bend opposite the pelvis.
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

	/// Same-sign pelvis yaw, hike, and whole-pelvis vertical bob.
	///
	/// `swing` is the per-leg thigh oscillator (stride timing). Yaw and bob use
	/// the global cycle so both hips share one parent-space turn.
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
		let gait = self.gait();
		let bob = self.body_bob();
		let leg = pose.leg_mut(side);
		leg.pelvis_turn = gait * hip_swing;
		let hike = swing * hip_lift * lift_sign;
		let vertical = bob * body_bob * side.sign();
		leg.pelvis_lateral = hike + vertical;
	}

	/// Neck nod/turn/tilt that counters lean, yaw, and side-bend.
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

	/// Inward shoulder/humerus on the forward arm, plus the shared swing/pump write.
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

	/// Elbow pump shared by walk and run; clip knobs differ.
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::{UprightRun, UprightWalk};
	use crate::rigs::humanoid::gait_knee::lerp_swing_knee;

	const PHASES: [f32; 9] = [0.0, 0.125, 0.25, 0.37, 0.5, 0.625, 0.75, 0.875, 0.93];

	#[test]
	fn body_bob_is_high_at_midstance() {
		assert!(GaitCoordination::from_progress(0.0).body_bob() > 0.99);
		assert!(GaitCoordination::from_progress(0.5).body_bob() > 0.99);
		assert!(GaitCoordination::from_progress(0.25).body_bob() < -0.99);
	}

	#[test]
	fn thigh_swing_is_a_unit_triangle() {
		assert!((GaitCoordination::thigh_swing(0.0) + 1.0).abs() < 1e-6);
		assert!(GaitCoordination::thigh_swing(0.25).abs() < 1e-6);
		assert!((GaitCoordination::thigh_swing(0.5) - 1.0).abs() < 1e-6);
		assert!(GaitCoordination::thigh_swing(0.75).abs() < 1e-6);
	}

	#[test]
	fn walk_pose_matches_frozen_pre_extract_computations() {
		let walk = UprightWalk::default();
		for phase in PHASES {
			assert_eq!(
				walk.sample_pose(phase),
				frozen_walk_pose(&walk, phase),
				"walk pose drifted at {phase}"
			);
		}
	}

	#[test]
	fn run_pose_matches_frozen_pre_extract_computations() {
		let run = UprightRun::default();
		for phase in PHASES {
			assert_eq!(
				run.sample_pose(phase),
				frozen_run_pose(&run, phase),
				"run pose drifted at {phase}"
			);
		}
	}

	#[test]
	fn walk_pose_matches_frozen_with_scaled_coordination_coeffs() {
		let walk = UprightWalk {
			torso_lean: 0.22,
			torso_turn: 0.31,
			side_lean: 0.17,
			hip_swing: 0.28,
			hip_lift: 0.19,
			body_bob: 0.14,
			arm_across: 0.21,
			head_stabilize: 1.0,
			shoulder_swing: 0.2,
			shoulder_lift: 0.11,
			humerus_swing_scale: 0.4,
			elbow_bend: 0.5,
			elbow_pump: 0.3,
			elbow_cycle: 0.12,
			..UprightWalk::default()
		};
		for phase in PHASES {
			assert_eq!(
				walk.sample_pose(phase),
				frozen_walk_pose(&walk, phase),
				"walk pose drifted at {phase} with scaled coeffs"
			);
		}
	}

	#[test]
	fn run_pose_matches_frozen_with_scaled_coordination_coeffs() {
		let run = UprightRun {
			torso_lean: 0.27,
			torso_turn: 0.33,
			side_lean: 0.12,
			hip_swing: 0.24,
			hip_lift: 0.15,
			body_bob: 0.16,
			arm_across: 0.23,
			head_stabilize: 0.4,
			shoulder_swing: 0.26,
			shoulder_lift: 0.13,
			humerus_swing_scale: 0.6,
			elbow_bend: 1.4,
			elbow_pump: 0.7,
			elbow_cycle: 0.4,
			..UprightRun::default()
		};
		for phase in PHASES {
			assert_eq!(
				run.sample_pose(phase),
				frozen_run_pose(&run, phase),
				"run pose drifted at {phase} with scaled coeffs"
			);
		}
	}

	/// Frozen copy of the #1344 walk sample (coordination + walk-only knee/stride/contact).
	fn frozen_walk_pose(walk: &UprightWalk, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let phase = Progress(progress).cycle();
		let gait = frozen_thigh_swing(phase);
		let left_arm_swing = -frozen_arm_swing(phase);
		let right_arm_swing = frozen_arm_swing(phase + 0.5);

		pose.apply_root(walk.torso_lean);
		pose.spine.add_turn(gait * walk.torso_turn);
		pose.spine.side_bend[1] += gait * walk.side_lean * 0.6;
		pose.spine.side_bend[2] += gait * walk.side_lean * 0.4;
		frozen_walk_leg(&mut pose, Side::Left, phase, -1.0, walk);
		frozen_walk_leg(&mut pose, Side::Right, phase, 1.0, walk);
		frozen_walk_arm(&mut pose, Side::Left, left_arm_swing, phase, -walk.arm_down, walk);
		frozen_walk_arm(&mut pose, Side::Right, right_arm_swing, phase, walk.arm_down, walk);
		frozen_apply_head(
			&mut pose,
			gait,
			walk.torso_lean,
			walk.torso_turn,
			walk.side_lean,
			walk.head_stabilize,
		);
		pose
	}

	fn frozen_walk_leg(
		pose: &mut HumanoidPose,
		side: Side,
		cycle: f32,
		lift_sign: f32,
		walk: &UprightWalk,
	) {
		let phase = if side == Side::Left { cycle } else { cycle + side.phase_offset() };
		let swing = frozen_thigh_swing(phase);
		let gait = frozen_thigh_swing(cycle);
		let bob = frozen_body_bob(cycle);
		let leg = pose.leg_mut(side);
		leg.pelvis_turn = gait * walk.hip_swing;
		let hike = frozen_hip_lift(swing, walk.hip_lift) * lift_sign;
		let vertical = bob * walk.body_bob * side.sign();
		leg.pelvis_lateral = hike + vertical;
		leg.hip_flexion = swing * walk.stride;
		leg.hip_abduction = -swing * walk.femur_medial_counter * lift_sign;
		leg.knee_flexion = lerp_swing_knee(phase, walk.knee_stance_bend, walk.knee_swing_bend);
		leg.pelvis_flexion = swing.max(0.0) * walk.foot_roll;
	}

	fn frozen_walk_arm(
		pose: &mut HumanoidPose,
		side: Side,
		arm_swing_value: f32,
		phase: f32,
		humerus_flex: f32,
		walk: &UprightWalk,
	) {
		let forward = (-side.sign() * arm_swing_value).max(0.0);
		let across = -side.sign() * forward * walk.arm_across;
		pose.apply_arm_twisted(
			side,
			arm_swing_value * walk.shoulder_swing,
			-frozen_shoulder_lift(arm_swing_value, walk.shoulder_lift) + across,
			across * 0.35,
			arm_swing_value * walk.humerus_swing_scale,
			humerus_flex + across * 0.45,
			frozen_elbow_flex(
				arm_swing_value,
				phase,
				-1.0,
				walk.elbow_bend,
				walk.elbow_pump,
				walk.elbow_cycle,
			),
		);
	}

	/// Frozen copy of the #1344 run sample (coordination + run-only knee/stride/contact).
	fn frozen_run_pose(run: &UprightRun, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let phase = Progress(progress).cycle();
		let gait = frozen_thigh_swing(phase);
		let left_arm_swing = -frozen_arm_swing(phase);
		let right_arm_swing = frozen_arm_swing(phase + 0.5);

		pose.apply_root(run.torso_lean);
		pose.spine.add_turn(gait * run.torso_turn);
		pose.spine.side_bend[1] += gait * run.side_lean * 0.6;
		pose.spine.side_bend[2] += gait * run.side_lean * 0.4;
		frozen_run_leg(&mut pose, Side::Left, phase, -1.0, run);
		frozen_run_leg(&mut pose, Side::Right, phase, 1.0, run);
		frozen_run_arm(&mut pose, Side::Left, left_arm_swing, phase, -run.arm_down, run);
		frozen_run_arm(&mut pose, Side::Right, right_arm_swing, phase, run.arm_down, run);
		frozen_apply_head(
			&mut pose,
			gait,
			run.torso_lean,
			run.torso_turn,
			run.side_lean,
			run.head_stabilize,
		);
		pose
	}

	fn frozen_run_leg(
		pose: &mut HumanoidPose,
		side: Side,
		cycle: f32,
		lift_sign: f32,
		run: &UprightRun,
	) {
		let phase = if side == Side::Left { cycle } else { cycle + 0.5 };
		let swing = frozen_thigh_swing(phase);
		let gait = frozen_thigh_swing(cycle);
		let bob = frozen_body_bob(cycle);
		let leg = pose.leg_mut(side);
		leg.pelvis_turn = gait * run.hip_swing;
		let hike = frozen_hip_lift(swing, run.hip_lift) * lift_sign;
		let vertical = bob * run.body_bob * side.sign();
		leg.pelvis_lateral = hike + vertical;
		leg.hip_flexion = swing * run.stride;
		leg.knee_flexion = frozen_run_knee_flex(phase, run) - run.knee_extended;
		leg.pelvis_flexion = swing.max(0.0) * run.foot_roll;
	}

	fn frozen_run_arm(
		pose: &mut HumanoidPose,
		side: Side,
		arm_swing_value: f32,
		phase: f32,
		humerus_flex: f32,
		run: &UprightRun,
	) {
		let forward = (-side.sign() * arm_swing_value).max(0.0);
		let across = -side.sign() * forward * run.arm_across;
		pose.apply_arm_twisted(
			side,
			arm_swing_value * run.shoulder_swing,
			-frozen_shoulder_lift(arm_swing_value, run.shoulder_lift) + across,
			across * 0.35,
			arm_swing_value * run.humerus_swing_scale,
			humerus_flex + across * 0.45,
			frozen_elbow_flex(
				arm_swing_value,
				phase,
				-1.0,
				run.elbow_bend,
				run.elbow_pump,
				run.elbow_cycle,
			),
		);
	}

	fn frozen_run_knee_flex(leg_phase: f32, run: &UprightRun) -> f32 {
		let p = leg_phase.fract();
		if p < 0.5 {
			let t = p * 2.0;
			run.knee_neutral + (t * PI).sin() * (run.knee_extended - run.knee_neutral)
		} else {
			lerp_swing_knee(p, run.knee_extended, run.knee_contracted)
		}
	}

	fn frozen_apply_head(
		pose: &mut HumanoidPose,
		gait: f32,
		torso_lean: f32,
		torso_turn: f32,
		side_lean: f32,
		stabilize: f32,
	) {
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

	fn frozen_body_bob(phase: f32) -> f32 {
		(phase * TAU * 2.0).cos()
	}

	fn frozen_thigh_swing(phase: f32) -> f32 {
		let p = phase.fract();
		if p < 0.5 {
			4.0 * p - 1.0
		} else {
			3.0 - 4.0 * p
		}
	}

	fn frozen_arm_swing(phase: f32) -> f32 {
		frozen_thigh_swing(phase) * 0.75
	}

	fn frozen_elbow_flex(
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

	fn frozen_shoulder_lift(arm_swing: f32, amplitude: f32) -> f32 {
		arm_swing * amplitude
	}

	fn frozen_hip_lift(leg_swing: f32, amplitude: f32) -> f32 {
		leg_swing * amplitude
	}
}
