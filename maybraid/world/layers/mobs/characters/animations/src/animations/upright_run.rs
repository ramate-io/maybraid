//! Biped upright run tuning parameters.
//!
//! [`UprightRun`] holds art-level humanoid knobs. Rig-agnostic callers use [`Run`](super::Run)
//! and convert via [`UprightRun::from_run`].

use std::f32::consts::PI;

use super::Run;

#[derive(Debug, Clone, PartialEq)]
pub struct UprightRun {
	pub arm_down: f32,
	pub elbow_bend: f32,
	pub elbow_pump: f32,
	pub elbow_cycle: f32,
	pub shoulder_swing: f32,
	pub shoulder_lift: f32,
	pub hip_swing: f32,
	pub hip_lift: f32,
	pub stride: f32,
	pub humerus_swing_scale: f32,
	/// Forward root lean; larger than walk so speed reads in the torso.
	pub torso_lean: f32,
	/// Shoulder-girdle yaw opposite the pelvis (radians at full stride).
	pub torso_turn: f32,
	/// Whole-pelvis vertical bob, twice per cycle, high at midstance (radians).
	pub body_bob: f32,
	/// Waist side-bend toward the stance foot (radians at full stride).
	pub side_lean: f32,
	/// Inward shoulder/humerus on the forward arm (radians at full forward swing).
	pub arm_across: f32,
	/// How much the neck counters lean, yaw, and side-bend (`0..=1`).
	pub head_stabilize: f32,
	/// Per-leg pelvis pitch on the trailing side (heel-to-toe without a foot bone).
	pub foot_roll: f32,
	pub knee_neutral: f32,
	pub knee_contracted: f32,
	pub knee_extended: f32,
}

impl Default for UprightRun {
	fn default() -> Self {
		Self {
			arm_down: 0.85,
			elbow_bend: 1.25,
			elbow_pump: 0.5,
			elbow_cycle: 0.35,
			shoulder_swing: 0.18,
			shoulder_lift: 0.08,
			hip_swing: 0.16,
			hip_lift: 0.08,
			stride: 1.05,
			humerus_swing_scale: 0.75,
			torso_lean: 0.18,
			torso_turn: 0.14,
			body_bob: 0.08,
			side_lean: 0.05,
			arm_across: 0.12,
			head_stabilize: 0.7,
			foot_roll: 0.10,
			knee_neutral: PI * 0.5,
			knee_contracted: 2.15,
			knee_extended: 0.35,
		}
	}
}

impl UprightRun {
	/// Scale the tuned upright template from rig-agnostic [`Run`] knobs.
	pub fn from_run(run: &Run) -> Self {
		let template = Self::default();
		let reference = Run::default();
		let stride_scale = run.stride / reference.stride;

		let knee_neutral_delta = template.knee_neutral - template.knee_extended;
		let knee_contracted_delta = template.knee_contracted - template.knee_neutral;

		Self {
			stride: template.stride * stride_scale,
			hip_lift: template.hip_lift * run.bounce,
			body_bob: template.body_bob * run.bounce,
			shoulder_lift: template.shoulder_lift * run.bounce,
			hip_swing: template.hip_swing * run.rotation,
			shoulder_swing: template.shoulder_swing * run.rotation,
			torso_lean: template.torso_lean * run.rotation,
			torso_turn: template.torso_turn * run.rotation,
			side_lean: template.side_lean * run.rotation,
			arm_across: template.arm_across * run.rotation,
			foot_roll: template.foot_roll * run.rotation,
			knee_extended: template.knee_extended,
			knee_neutral: template.knee_extended + knee_neutral_delta * stride_scale,
			knee_contracted: template.knee_extended
				+ knee_neutral_delta * stride_scale
				+ knee_contracted_delta * stride_scale,
			arm_down: template.arm_down,
			elbow_bend: template.elbow_bend,
			elbow_pump: template.elbow_pump,
			elbow_cycle: template.elbow_cycle,
			humerus_swing_scale: template.humerus_swing_scale,
			head_stabilize: template.head_stabilize,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_run_default_matches_template() {
		let run = Run::default();
		assert_eq!(UprightRun::from_run(&run), UprightRun::default());
	}

	#[test]
	fn from_run_scales_stride_and_knee_amplitude() {
		let run = Run { stride: 2.1, ..Run::default() };
		let upright = UprightRun::from_run(&run);
		let template = UprightRun::default();
		assert!((upright.stride - template.stride * 2.0).abs() < 1e-5);
		assert!(
			(upright.knee_contracted - template.knee_contracted * 2.0 + template.knee_extended)
				.abs() < 1e-4
		);
	}

	#[test]
	fn from_run_scales_bounce_and_rotation() {
		let run = Run { bounce: 2.0, rotation: 0.5, ..Run::default() };
		let upright = UprightRun::from_run(&run);
		let template = UprightRun::default();
		assert!((upright.hip_lift - template.hip_lift * 2.0).abs() < 1e-5);
		assert!((upright.body_bob - template.body_bob * 2.0).abs() < 1e-5);
		assert!((upright.hip_swing - template.hip_swing * 0.5).abs() < 1e-5);
		assert!((upright.shoulder_swing - template.shoulder_swing * 0.5).abs() < 1e-5);
		assert!((upright.torso_lean - template.torso_lean * 0.5).abs() < 1e-5);
	}
}
