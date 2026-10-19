//! One-shot victory wave: raise one arm out and up, bend the elbow back
//! toward the head, oscillate the forearm side-to-side, recover.
//!
//! Progress is clamped to `[0.0, 1.0]`. Sampling is repeatable from effective rest,
//! clip parameters, and progress only.

use std::f32::consts::{FRAC_PI_6, TAU};

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

const RAISE_END: f32 = 0.28;
const WAVE_END: f32 = 0.82;
const WAVE_CYCLES: f32 = 2.5;

/// Character-space humerus weights before normalize. Own-side lateral is
/// [`Side::sign`]: right −X, left +X. Not a vertical overhead salute.
const AIM_UP: f32 = 0.70;
const AIM_OUTBOARD: f32 = 0.68;
const AIM_FORWARD: f32 = 0.18;
const WAVE_LATERAL_AMP: f32 = 0.22;
const WAVE_FORWARD_AMP: f32 = 0.05;

/// Elbow closes enough that the forearm comes back toward the midline.
const RAISE_ELBOW: f32 = 1.2;
const WAVE_ELBOW_AMP: f32 = 0.10;
const SHOULDER_CARRY: f32 = 0.1;

/// Long-axis roll after aim. Measured on `for_clip_test()` so both elbows
/// fold the forearm medially and slightly fight-forward — not the jab π/2 punch plane.
const WAVE_HUMERUS_ROLL: f32 = FRAC_PI_6;

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
		if !(RAISE_END..=WAVE_END).contains(&t) {
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
		let rest = Vec3::new(side.sign(), 0.0, 0.0);
		if raise < 1e-4 {
			return rest;
		}
		let wave = self.wave_oscillation(progress);
		let raised = Vec3::new(
			side.sign() * (AIM_OUTBOARD + WAVE_LATERAL_AMP * wave),
			AIM_UP,
			AIM_FORWARD + WAVE_FORWARD_AMP * wave.abs(),
		)
		.normalize();
		rest.lerp(raised, raise).normalize()
	}

	/// Roll about the humerus length so the bent elbow sends the forearm back medially.
	pub fn humerus_roll(&self, side: Side, _progress: f32) -> f32 {
		WAVE_HUMERUS_ROLL * side.sign()
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
		let along = wave.humerus_along(Side::Right, PEAK);
		assert!(along.y > 0.5, "humerus still lifts, {along:?}");
		assert!(along.y < 0.88, "humerus must not stand near vertical, {along:?}");
		assert!(along.x < -0.35, "right humerus aims out on its own side, {along:?}");
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

	#[test]
	fn victory_wave_humerus_aim_mirrors_outboard() -> anyhow::Result<()> {
		let right = VictoryWave::new(Side::Right).humerus_along(Side::Right, PEAK);
		let left = VictoryWave::new(Side::Left).humerus_along(Side::Left, PEAK);
		assert!(right.x < -0.35, "right aim is own-side outboard, {right:?}");
		assert!(left.x > 0.35, "left aim is own-side outboard, {left:?}");
		assert!((right.x + left.x).abs() < 1e-4, "aim X mirrors, {right:?} vs {left:?}");
		assert!((right.y - left.y).abs() < 1e-4, "aim Y matches, {right:?} vs {left:?}");
		Ok(())
	}
}
