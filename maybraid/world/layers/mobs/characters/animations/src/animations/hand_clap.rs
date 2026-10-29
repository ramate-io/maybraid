//! Bilateral chest clap: sweep both hands to a shared chest target, pulse apart, recover.
//!
//! Targets are relative to the chest landmark. The rig adapter uses the bound
//! body mesh's terminal points and full affine transforms, with fixed Idle(0)
//! for the approach and recovery baseline.

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

const SWEEP_END: f32 = 0.30;
const CLAP_END: f32 = 0.78;
const CLAP_PULSES: f32 = 2.0;

const CHEST_TARGET_OFFSET: Vec3 = Vec3::new(0.0, 0.0, 0.40);
const MAX_HAND_GAP: f32 = 0.14;

/// Short celebration clap in front of the chest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HandClap;

impl HandClap {
	/// Smooth sweep and recovery envelope for the bilateral gesture.
	pub fn clap_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t <= SWEEP_END {
			smoothstep(t / SWEEP_END)
		} else if t <= CLAP_END {
			1.0
		} else {
			1.0 - smoothstep((t - CLAP_END) / (1.0 - CLAP_END))
		}
	}

	/// Opening amount of the two contact pulses; zero is palm contact.
	pub fn clap_pulse(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if !(SWEEP_END..=CLAP_END).contains(&t) {
			return 0.0;
		}
		let u = (t - SWEEP_END) / (CLAP_END - SWEEP_END);
		let edge = 0.12;
		let envelope = if u < edge {
			smoothstep(u / edge)
		} else if u > 1.0 - edge {
			smoothstep((1.0 - u) / edge)
		} else {
			1.0
		};
		0.5 * (1.0 - (std::f32::consts::TAU * CLAP_PULSES * u).cos()) * envelope
	}

	/// Symmetric shoulder carry around the bind; left/right signs follow `Side::sign`.
	pub fn shoulder_carry(&self, side: Side, progress: f32) -> f32 {
		0.10 * self.clap_amount(progress) * side.sign()
	}

	/// Hand target offset from `upper_back` in character space.
	pub fn hand_target(&self, side: Side, progress: f32) -> Vec3 {
		CHEST_TARGET_OFFSET
			+ Vec3::X * (side.sign() * MAX_HAND_GAP * self.clap_pulse(progress) * 0.5)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_rigs::Side;

	const CONTACT: f32 = (SWEEP_END + CLAP_END) * 0.5;

	#[test]
	fn envelope_clamps_and_returns_to_fixed_idle_phase_zero() -> anyhow::Result<()> {
		let clap = HandClap;
		assert_eq!(clap.clap_amount(-0.5), 0.0);
		assert_eq!(clap.clap_amount(1.5), 0.0);
		assert_eq!(clap.hand_target(Side::Left, 0.0), clap.hand_target(Side::Right, 1.0));
		Ok(())
	}

	#[test]
	fn pulse_opens_twice_and_closes_on_contact_beats() -> anyhow::Result<()> {
		let clap = HandClap;
		assert!(clap.clap_pulse(SWEEP_END).abs() < 1e-6);
		assert!(clap.clap_pulse(CLAP_END).abs() < 1e-6);
		let first_open = SWEEP_END + (CLAP_END - SWEEP_END) * 0.25;
		let second_open = SWEEP_END + (CLAP_END - SWEEP_END) * 0.75;
		assert!(clap.clap_pulse(first_open) > 0.9);
		assert!(clap.clap_pulse(second_open) > 0.9);
		Ok(())
	}

	#[test]
	fn approach_and_recovery_are_fine_boundary_continuous() -> anyhow::Result<()> {
		let clap = HandClap;
		for boundary in [SWEEP_END, CLAP_END] {
			let epsilon = 1e-4;
			let before = clap.clap_amount(boundary - epsilon);
			let at = clap.clap_amount(boundary);
			let after = clap.clap_amount(boundary + epsilon);
			assert!((at - before).abs() < 2e-5 && (after - at).abs() < 2e-5);
			for side in [Side::Left, Side::Right] {
				let a = clap.hand_target(side, boundary - epsilon);
				let b = clap.hand_target(side, boundary);
				let c = clap.hand_target(side, boundary + epsilon);
				assert!((b - a).length() < 0.002 && (c - b).length() < 0.002);
				assert!(clap.hand_target(side, boundary - epsilon).is_finite());
			}
		}
		Ok(())
	}

	#[test]
	fn hand_targets_open_and_mirror_by_side() -> anyhow::Result<()> {
		let clap = HandClap;
		let left = clap.hand_target(Side::Left, 0.42);
		let right = clap.hand_target(Side::Right, 0.42);
		assert!((left.x + right.x).abs() < 1e-5);
		assert!((left.y - right.y).abs() < 1e-5);
		assert!((left.z - right.z).abs() < 1e-5);
		assert!(left.x > 0.0 && right.x < 0.0);
		assert!(
			(clap.shoulder_carry(Side::Left, CONTACT) + clap.shoulder_carry(Side::Right, CONTACT))
				.abs() < 1e-6
		);
		Ok(())
	}

	#[test]
	fn out_of_order_sampling_uses_only_progress() -> anyhow::Result<()> {
		let clap = HandClap;
		let late = clap.hand_target(Side::Left, 0.72);
		let _ = clap.hand_target(Side::Left, 0.12);
		assert_eq!(late, clap.hand_target(Side::Left, 0.72));
		Ok(())
	}
}
