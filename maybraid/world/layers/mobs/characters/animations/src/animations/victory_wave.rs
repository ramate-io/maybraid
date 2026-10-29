//! One-shot victory wave: raise one arm out and up, bend the elbow back
//! toward the head, oscillate the forearm side-to-side, recover.
//!
//! Progress clamps to `[0.0, 1.0]`; sampling depends only on this clip and the
//! requested progress. The clip leaves root effects at identity.

use std::f32::consts::{FRAC_PI_6, TAU};

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

/// End of the lift. The wave is held through [`WAVE_END`].
pub const RAISE_END: f32 = 0.28;
/// Start of the smooth recovery.
pub const WAVE_END: f32 = 0.82;
const WAVE_CYCLES: f32 = 2.5;

/// Character-space humerus weights before normalization. Own-side lateral is
/// [`Side::sign`]: right −X, left +X.
const AIM_UP: f32 = 0.70;
const AIM_OUTBOARD: f32 = 0.68;
const AIM_FORWARD: f32 = 0.18;
const WAVE_LATERAL_AMP: f32 = 0.22;
const WAVE_FORWARD_AMP: f32 = 0.05;

const RAISE_ELBOW: f32 = 1.2;
const WAVE_ELBOW_AMP: f32 = 0.10;
const SHOULDER_CARRY: f32 = 0.1;
/// Measured on the production bind: folds both forearms medially and upward.
const WAVE_HUMERUS_ROLL: f32 = FRAC_PI_6;

const TORSO_TURN: f32 = 0.14;
const KNEE_BOUNCE: f32 = 0.09;
const NECK_NOD: f32 = 0.1;

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

	/// Lift envelope: up through the wave, down on recovery.
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

	/// Side-to-side oscillation during the held wave; zero outside its window.
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

	/// Fully raised character-space humerus direction. The rig adapter blends
	/// the resolved target from neutral Idle using [`Self::raise_amount`].
	pub fn humerus_along(&self, side: Side, progress: f32) -> Vec3 {
		let wave = self.wave_oscillation(progress);
		Vec3::new(
			side.sign() * (AIM_OUTBOARD + WAVE_LATERAL_AMP * wave),
			AIM_UP,
			AIM_FORWARD + WAVE_FORWARD_AMP * wave.abs(),
		)
		.normalize()
	}

	/// Long-axis roll, tuned for a bent medial/upward forearm on both binds.
	pub fn humerus_roll(&self, side: Side, _progress: f32) -> f32 {
		WAVE_HUMERUS_ROLL * side.sign()
	}

	/// Target elbow bend before the Idle-to-target pose blend.
	pub fn elbow_flex(&self, progress: f32) -> f32 {
		let wave = self.wave_oscillation(progress);
		RAISE_ELBOW + WAVE_ELBOW_AMP * wave.abs()
	}

	pub fn shoulder_carry(&self, _progress: f32) -> f32 {
		SHOULDER_CARRY
	}

	pub fn torso_turn(&self, _progress: f32) -> f32 {
		TORSO_TURN * -self.side.sign()
	}

	pub fn knee_bounce(&self, progress: f32) -> f32 {
		KNEE_BOUNCE * (1.0 + 0.35 * self.wave_oscillation(progress).abs())
	}

	pub fn neck_nod(&self, _progress: f32) -> f32 {
		NECK_NOD
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const WAVE_EXTREME_A: f32 = RAISE_END + 0.3 * (WAVE_END - RAISE_END);
	const WAVE_HOLD: f32 = (RAISE_END + WAVE_END) * 0.5;
	const WAVE_EXTREME_B: f32 = RAISE_END + 0.7 * (WAVE_END - RAISE_END);

	#[test]
	fn victory_wave_has_clamped_rest_endpoints_and_smooth_envelope() -> anyhow::Result<()> {
		let wave = VictoryWave::default();
		assert_eq!(wave.raise_amount(-0.5), wave.raise_amount(0.0));
		assert_eq!(wave.raise_amount(1.5), wave.raise_amount(1.0));
		assert!(wave.raise_amount(0.0).abs() < 1e-6);
		assert!(wave.raise_amount(1.0).abs() < 1e-6);
		assert!(wave.raise_amount(RAISE_END).abs() > 0.99);
		assert!(wave.raise_amount(WAVE_END).abs() > 0.99);
		assert!(wave.wave_oscillation(RAISE_END).abs() < 1e-6);
		assert!(wave.wave_oscillation(WAVE_END).abs() < 1e-6);
		assert!(wave.elbow_flex(WAVE_HOLD) > 1.0);
		assert!(wave.shoulder_carry(WAVE_HOLD) > 0.0);
		Ok(())
	}

	#[test]
	fn victory_wave_lifts_then_sweeps_and_mirrors_aim() -> anyhow::Result<()> {
		let wave = VictoryWave::default();
		assert!(wave.wave_oscillation(RAISE_END * 0.5).abs() < 1e-6);
		for progress in [WAVE_EXTREME_A, WAVE_HOLD, WAVE_EXTREME_B] {
			let right = wave.humerus_along(Side::Right, progress);
			let left = wave.humerus_along(Side::Left, progress);
			assert!(right.x < -0.3, "right humerus points outboard: {right:?}");
			assert!(left.x > 0.3, "left humerus points outboard: {left:?}");
			assert!(right.y > 0.5 && right.y < 0.88, "right humerus lifts: {right:?}");
			assert!((right.x + left.x).abs() < 1e-5);
			assert!((right.y - left.y).abs() < 1e-5);
			assert!((right.z - left.z).abs() < 1e-5);
		}
		assert!(
			(wave.humerus_along(Side::Right, WAVE_EXTREME_A).x
				- wave.humerus_along(Side::Right, WAVE_HOLD).x)
				.abs() > 0.05
		);
		Ok(())
	}

	#[test]
	fn mirrored_torso_and_humerus_roll_follow_side_sign() -> anyhow::Result<()> {
		let right = VictoryWave::new(Side::Right);
		let left = VictoryWave::new(Side::Left);
		assert!((right.torso_turn(WAVE_HOLD) + left.torso_turn(WAVE_HOLD)).abs() < 1e-5);
		assert!(
			(right.humerus_roll(Side::Right, WAVE_HOLD) + left.humerus_roll(Side::Left, WAVE_HOLD))
				.abs() < 1e-5
		);
		Ok(())
	}
}
