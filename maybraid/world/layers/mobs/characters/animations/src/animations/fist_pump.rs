//! Victory fist pump: chamber one arm, snap the fist overhead, and recover.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

pub const RAISE_END: f32 = 0.24;
pub const HOLD_END: f32 = 0.62;
const CHAMBER_ELBOW: f32 = 1.55;
const PEAK_ELBOW: f32 = 0.95;
const PUMP_ROLL: f32 = FRAC_PI_2;
const ROOT_LEAN_BACK: f32 = 0.10;
const STANCE_KNEE: f32 = 0.12;
const SHOULDER_WINDUP: f32 = 0.20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FistPump {
	pub side: Side,
}

impl Default for FistPump {
	fn default() -> Self {
		Self { side: Side::Right }
	}
}

impl FistPump {
	pub fn new(side: Side) -> Self {
		Self { side }
	}

	pub fn with_side(mut self, side: Side) -> Self {
		self.side = side;
		self
	}

	pub fn opposite_side(&self) -> Side {
		self.side.opposite()
	}

	pub fn pump_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < RAISE_END {
			smoothstep(t / RAISE_END)
		} else if t < HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - HOLD_END) / (1.0 - HOLD_END))
		}
	}

	pub fn raise_mix(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t >= RAISE_END {
			return 1.0;
		}
		let u = t / RAISE_END;
		if u < 0.4 {
			smoothstep(u / 0.4) * 0.35
		} else {
			0.35 + smoothstep((u - 0.4) / 0.6) * 0.65
		}
	}

	pub fn humerus_along(&self, progress: f32) -> Vec3 {
		let mix = self.raise_mix(progress);
		let outboard = self.side.sign() * 0.38;
		let chamber = Vec3::new(outboard * 0.35, -0.22, -0.62);
		let peak = Vec3::new(outboard, 0.88, 0.12);
		chamber.lerp(peak, mix).normalize()
	}

	pub fn pump_roll(&self) -> f32 {
		PUMP_ROLL * self.side.sign()
	}

	pub fn elbow_flex(&self, progress: f32) -> f32 {
		CHAMBER_ELBOW + (PEAK_ELBOW - CHAMBER_ELBOW) * self.raise_mix(progress)
	}

	pub fn shoulder_windup(&self, progress: f32) -> f32 {
		-SHOULDER_WINDUP * (1.0 - self.raise_mix(progress))
	}

	pub fn root_lean_back(&self, progress: f32) -> f32 {
		-ROOT_LEAN_BACK * self.raise_mix(progress)
	}

	pub fn stance_knee(&self, progress: f32) -> f32 {
		STANCE_KNEE * self.raise_mix(progress)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fist_pump_clamps_and_transitions_from_chamber_to_overhead() -> anyhow::Result<()> {
		let right = FistPump::new(Side::Right);
		let left = FistPump::new(Side::Left);
		assert_eq!(right.pump_amount(-1.0), 0.0);
		assert_eq!(right.pump_amount(2.0), 0.0);
		assert!(right.pump_amount(0.0) < 1e-6 && right.pump_amount(1.0) < 1e-6);
		assert!(right.pump_amount(0.43) > 0.99);
		let chamber = right.humerus_along(0.0);
		let peak = right.humerus_along(0.24);
		assert!(chamber.y < 0.0 && chamber.z < 0.0);
		assert!(peak.y > 0.65 && peak.z > 0.0);
		let l = left.humerus_along(0.24);
		assert!((peak.x + l.x).abs() < 1e-6);
		assert!((right.pump_roll() + left.pump_roll()).abs() < 1e-6);
		assert!(right.elbow_flex(0.0) > right.elbow_flex(0.24));
		Ok(())
	}
}
