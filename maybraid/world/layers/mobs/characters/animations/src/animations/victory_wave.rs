//! One-shot victory wave: raise one arm overhead, oscillate side-to-side, recover.
//!
//! Progress is clamped to `[0.0, 1.0]`. Sampling is repeatable from effective rest,
//! clip parameters, and progress only.

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

const RAISE_END: f32 = 0.28;
const WAVE_END: f32 = 0.82;
const WAVE_CYCLES: f32 = 2.5;

const AIM_UP: f32 = 0.92;
const AIM_OUTBOARD: f32 = 0.28;
const AIM_FORWARD: f32 = 0.08;
const WAVE_LATERAL_AMP: f32 = 0.28;
const WAVE_FORWARD_AMP: f32 = 0.06;

const RAISE_ELBOW: f32 = 0.48;
const WAVE_ELBOW_AMP: f32 = 0.12;
const SHOULDER_CARRY: f32 = 0.1;

const TORSO_TURN: f32 = 0.14;
const KNEE_BOUNCE: f32 = 0.09;
const NECK_NOD: f32 = 0.1;
const COUNTER_SHOULDER: f32 = 0.18;

/// Short celebration wave on one arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VictoryWave {
	pub side: Side,
}

impl Default for VictoryWave {
	fn default() -> Self {
		Self { side: Side::Right }
	}
}

impl VictoryWave {
	pub fn new(side: Side) -> Self {
		Self { side }
	}

	pub fn with_side(mut self, side: Side) -> Self {
		self.side = side;
		self
	}

	/// Lift envelope: up through the wave, down on recover.
	pub fn raise_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t <= RAISE_END {
			smoothstep(t / RAISE_END)
		} else if t <= WAVE_END {
			1.0
		} else {
			1.0 - smoothstep((t - WAVE_END) / (1.0 - WAVE_END))
		}
	}

	/// Side-to-side oscillation during the held wave. Zero outside the wave window.
	pub fn wave_oscillation(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < RAISE_END || t > WAVE_END {
			return 0.0;
		}
		let u = (t - RAISE_END) / (WAVE_END - RAISE_END);
		let edge = 0.18;
		let envelope = if u < edge {
			smoothstep(u / edge)
		} else if u > 1.0 - edge {
			smoothstep((1.0 - u) / edge)
		} else {
			1.0
		};
		(TAU * WAVE_CYCLES * u).sin() * envelope
	}

	/// Character-space humerus aim. The resolver aims with this vector and roll.
	pub fn humerus_along(&self, side: Side, progress: f32) -> Vec3 {
		let raise = self.raise_amount(progress);
		if raise < 1e-4 {
			return Vec3::Y;
		}
		let wave = self.wave_oscillation(progress);
		let outboard = -side.sign() * AIM_OUTBOARD;
		let lateral = outboard * raise + WAVE_LATERAL_AMP * wave * side.sign();
		let up = 0.12 + AIM_UP * raise;
		let forward = AIM_FORWARD * raise + WAVE_FORWARD_AMP * wave.abs();
		Vec3::new(lateral, up, forward).normalize()
	}

	/// Roll about the humerus length so the elbow hinge opens toward the body front.
	pub fn humerus_roll(&self, side: Side, _progress: f32) -> f32 {
		FRAC_PI_2 * side.sign()
	}

	pub fn elbow_flex(&self, progress: f32) -> f32 {
		let raise = self.raise_amount(progress);
		let wave = self.wave_oscillation(progress);
		RAISE_ELBOW * raise + WAVE_ELBOW_AMP * wave.abs()
	}

	pub fn shoulder_carry(&self, progress: f32) -> f32 {
		SHOULDER_CARRY * self.raise_amount(progress)
	}

	pub fn torso_turn(&self, progress: f32) -> f32 {
		TORSO_TURN * self.raise_amount(progress) * -self.side.sign()
	}

	pub fn knee_bounce(&self, progress: f32) -> f32 {
		KNEE_BOUNCE
			* self.raise_amount(progress)
			* (1.0 + 0.35 * self.wave_oscillation(progress).abs())
	}

	pub fn neck_nod(&self, progress: f32) -> f32 {
		NECK_NOD * self.raise_amount(progress)
	}

	pub fn counter_shoulder(&self, progress: f32) -> f32 {
		-COUNTER_SHOULDER * self.raise_amount(progress) * self.side.sign()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const PEAK: f32 = (RAISE_END + WAVE_END) * 0.5;

	#[test]
	fn victory_wave_is_quiet_at_start_and_end() -> anyhow::Result<()> {
		let wave = VictoryWave::default();
		assert!(wave.raise_amount(0.0) < 1e-4);
		assert!(wave.raise_amount(1.0) < 1e-4);
		assert!(wave.wave_oscillation(0.0).abs() < 1e-4);
		assert!(wave.wave_oscillation(1.0).abs() < 1e-4);
		assert!(wave.elbow_flex(0.0).abs() < 1e-4);
		assert!(wave.elbow_flex(1.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn victory_wave_raises_before_oscillating() -> anyhow::Result<()> {
		let wave = VictoryWave::default();
		let mid_raise = RAISE_END * 0.5;
		assert!(wave.raise_amount(mid_raise) > 0.4);
		assert!(wave.wave_oscillation(mid_raise).abs() < 1e-4);
		assert!(wave.humerus_along(Side::Right, PEAK).y > 0.8);
		Ok(())
	}

	#[test]
	fn victory_wave_oscillates_during_hold() -> anyhow::Result<()> {
		let wave = VictoryWave::default();
		let a = wave.humerus_along(Side::Right, 0.40);
		let b = wave.humerus_along(Side::Right, 0.60);
		assert!((a.x - b.x).abs() > 0.05, "aim lateral must move, {a:?} vs {b:?}");
		Ok(())
	}

	#[test]
	fn victory_wave_is_repeatable() -> anyhow::Result<()> {
		let wave = VictoryWave::default();
		let progress = 0.47;
		assert!((wave.elbow_flex(progress) - wave.elbow_flex(progress)).abs() < 1e-6);
		assert!(
			(wave.humerus_along(Side::Right, progress) - wave.humerus_along(Side::Right, progress))
				.length() < 1e-6
		);
		Ok(())
	}

	#[test]
	fn victory_wave_mirrors_torso_turn() -> anyhow::Result<()> {
		let right = VictoryWave::new(Side::Right);
		let left = VictoryWave::new(Side::Left);
		assert!(right.torso_turn(PEAK) > 0.0);
		assert!(left.torso_turn(PEAK) < 0.0);
		assert!(
			(right.torso_turn(PEAK) + left.torso_turn(PEAK)).abs() < 1e-4,
			"torso turn mirrors across sides"
		);
		Ok(())
	}
}
