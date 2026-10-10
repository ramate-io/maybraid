//! Bilateral chest clap: sweep both forearms inward, pulse at contact, recover.
//!
//! Progress is clamped to `[0.0, 1.0]`. Sampling is repeatable from effective rest,
//! clip parameters, and progress only.

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::prelude::Vec3;

use crate::animations::smoothstep;
use crate::Progress;

const SWEEP_END: f32 = 0.30;
const CLAP_END: f32 = 0.78;
const CLAP_PULSES: f32 = 2.25;

const AIM_INBOARD: f32 = 0.22;
const AIM_UP: f32 = 0.48;
const AIM_FORWARD: f32 = 0.41;

const CLAP_ELBOW: f32 = 0.68;
const PULSE_ELBOW_AMP: f32 = 0.18;
const SHOULDER_CARRY: f32 = 0.14;

const SPINE_PITCH: f32 = 0.12;
const NECK_NOD: f32 = 0.06;

/// Short celebration clap in front of the chest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HandClap;

impl HandClap {
	/// Sweep / hold / recover envelope for the whole gesture.
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

	/// Small elbow pulses while the hands are held together.
	pub fn clap_pulse(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < SWEEP_END || t > CLAP_END {
			return 0.0;
		}
		let u = (t - SWEEP_END) / (CLAP_END - SWEEP_END);
		let edge = 0.15;
		let envelope = if u < edge {
			smoothstep(u / edge)
		} else if u > 1.0 - edge {
			smoothstep((1.0 - u) / edge)
		} else {
			1.0
		};
		(TAU * CLAP_PULSES * u).sin() * envelope
	}

	/// Character-space humerus aim toward the chest center.
	pub fn humerus_along(&self, side: character_rigs::Side, progress: f32) -> Vec3 {
		let amount = self.clap_amount(progress);
		if amount < 1e-4 {
			return Vec3::Y;
		}
		let inboard = -side.sign() * AIM_INBOARD;
		let up = 0.18 + AIM_UP * amount;
		let forward = AIM_FORWARD * amount;
		Vec3::new(inboard * amount, up, forward).normalize()
	}

	/// Roll so the elbow hinge opens toward the fight plane once aimed.
	pub fn humerus_roll(&self, side: character_rigs::Side, _progress: f32) -> f32 {
		FRAC_PI_2 * side.sign()
	}

	pub fn elbow_flex(&self, progress: f32) -> f32 {
		let amount = self.clap_amount(progress);
		let pulse = self.clap_pulse(progress);
		CLAP_ELBOW * amount + PULSE_ELBOW_AMP * pulse.abs()
	}

	/// Per-side shoulder flex so both clavicles rise symmetrically (see [`Fall::shoulder_flex`](super::fall::Fall::shoulder_flex)).
	pub fn shoulder_carry(&self, side: character_rigs::Side, progress: f32) -> f32 {
		SHOULDER_CARRY * self.clap_amount(progress) * side.sign()
	}

	pub fn spine_pitch(&self, progress: f32) -> f32 {
		SPINE_PITCH * self.clap_amount(progress)
	}

	pub fn neck_nod(&self, progress: f32) -> f32 {
		NECK_NOD * self.clap_amount(progress)
	}
}

#[cfg(test)]
mod tests {
	use character_rigs::Side;

	use super::*;

	const PEAK: f32 = (SWEEP_END + CLAP_END) * 0.5;

	#[test]
	fn hand_clap_is_quiet_at_loop_boundaries() -> anyhow::Result<()> {
		let clap = HandClap;
		assert!(clap.clap_amount(0.0) < 1e-4);
		assert!(clap.clap_amount(1.0) < 1e-4);
		assert!(clap.elbow_flex(0.0).abs() < 1e-4);
		assert!(clap.elbow_flex(1.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn hand_clap_sweeps_before_pulsing() -> anyhow::Result<()> {
		let clap = HandClap;
		let mid_sweep = SWEEP_END * 0.5;
		assert!(clap.clap_amount(mid_sweep) > 0.35);
		assert!(clap.clap_pulse(mid_sweep).abs() < 1e-4);
		assert!(
			clap.humerus_along(Side::Right, PEAK).x.abs() > 0.18,
			"humerus aim carries lateral component at peak"
		);
		Ok(())
	}

	#[test]
	fn hand_clap_pulses_during_hold() -> anyhow::Result<()> {
		let clap = HandClap;
		let a = clap.elbow_flex(0.38);
		let b = clap.elbow_flex(0.52);
		assert!((a - b).abs() > 0.02, "elbow should pulse during hold, {a} vs {b}");
		Ok(())
	}

	#[test]
	fn hand_clap_is_repeatable() -> anyhow::Result<()> {
		let clap = HandClap;
		let progress = 0.51;
		assert!((clap.elbow_flex(progress) - clap.elbow_flex(progress)).abs() < 1e-6);
		assert!(
			(clap.humerus_along(Side::Left, progress) - clap.humerus_along(Side::Left, progress))
				.length() < 1e-6
		);
		Ok(())
	}

	#[test]
	fn hand_clap_shoulder_carry_mirrors_by_side() -> anyhow::Result<()> {
		let clap = HandClap;
		let left = clap.shoulder_carry(Side::Left, PEAK);
		let right = clap.shoulder_carry(Side::Right, PEAK);
		assert!(left.abs() > 0.05);
		assert!(left.signum() != right.signum());
		Ok(())
	}

	#[test]
	fn hand_clap_mirrors_humerus_aim() -> anyhow::Result<()> {
		let clap = HandClap;
		let right = clap.humerus_along(Side::Right, PEAK);
		let left = clap.humerus_along(Side::Left, PEAK);
		assert!((right.x + left.x).abs() < 0.08, "lateral aim mirrors, {right:?} {left:?}");
		assert!((right.x.abs() - left.x.abs()).abs() < 0.08, "aim magnitude matches");
		Ok(())
	}
}
