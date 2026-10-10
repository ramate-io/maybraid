//! Victory fist pump: chamber low, snap the fist overhead, hold, recover.
//!
//! Distinct from overhead waves and bilateral claps: one arm drives a vertical
//! pump while the opposite arm and legs stay near rest. Body-space aim matches
//! [`Jab`](super::Jab): +X right, +Y up, +Z fight-forward.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

const RAISE_END: f32 = 0.24;
const HOLD_END: f32 = 0.62;

const CHAMBER_ELBOW: f32 = 1.55;
const PEAK_ELBOW: f32 = 0.95;
const PUMP_ROLL: f32 = FRAC_PI_2;
const ROOT_LEAN_BACK: f32 = 0.10;
const STANCE_KNEE: f32 = 0.12;
const SHOULDER_WINDUP: f32 = 0.20;

/// One-shot fist-pump knobs. The rig resolver maps these onto V0 channels.
#[derive(Debug, Clone, Copy, PartialEq)]
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

	/// Raise, hold, recover envelope in `[0, 1]`.
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

	/// `0` at entry, `1` at full overhead pose (within the raise window).
	fn raise_mix(&self, progress: f32) -> f32 {
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
		// Match [`Jab::humerus_lateral`]: left → −X, right → +X in character space.
		let outboard = -self.side.sign() * 0.38;
		let chamber = Vec3::new(-outboard * 0.35, -0.22, -0.62);
		// Peak lateral matches chamber sign so the fist stays on the pumping side at full extend.
		let peak = Vec3::new(-outboard, 0.88, 0.12);
		chamber.lerp(peak, mix).normalize_or_zero()
	}

	pub fn pump_roll(&self) -> f32 {
		PUMP_ROLL
	}

	pub fn elbow_flex(&self, progress: f32) -> f32 {
		let mix = self.raise_mix(progress);
		(CHAMBER_ELBOW + (PEAK_ELBOW - CHAMBER_ELBOW) * mix) * self.pump_amount(progress)
	}

	pub fn shoulder_windup(&self, progress: f32) -> f32 {
		let amount = self.pump_amount(progress);
		let windup = 1.0 - self.raise_mix(progress);
		-SHOULDER_WINDUP * amount * windup
	}

	pub fn root_lean_back(&self, progress: f32) -> f32 {
		-ROOT_LEAN_BACK * self.pump_amount(progress) * self.raise_mix(progress)
	}

	pub fn stance_knee(&self, progress: f32) -> f32 {
		STANCE_KNEE * self.pump_amount(progress) * self.raise_mix(progress)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn peak() -> f32 {
		(RAISE_END + HOLD_END) * 0.5
	}

	#[test]
	fn fist_pump_starts_and_ends_at_rest() -> anyhow::Result<()> {
		let pump = FistPump::default();
		assert!(pump.pump_amount(0.0) < 1e-4);
		assert!(pump.pump_amount(1.0) < 1e-4);
		assert!(pump.elbow_flex(0.0) < 1e-4);
		assert!(pump.elbow_flex(1.0) < 0.05);
		Ok(())
	}

	#[test]
	fn fist_pump_reaches_full_hold() -> anyhow::Result<()> {
		let pump = FistPump::default();
		assert!(pump.pump_amount(peak()) > 0.99);
		let along = pump.humerus_along(peak());
		assert!(along.y > 0.75, "overhead aim, got {along:?}");
		assert!(along.z > 0.0, "slight forward carry, got {along:?}");
		Ok(())
	}

	#[test]
	fn fist_pump_outboard_sign_follows_side() -> anyhow::Result<()> {
		let right = FistPump::default().with_side(Side::Right);
		let left = FistPump::default().with_side(Side::Left);
		let r = right.humerus_along(peak());
		let l = left.humerus_along(peak());
		assert!(r.x < -0.15, "right pump aim stays on −X, got {r:?}");
		assert!(l.x > 0.15, "left pump aim stays on +X, got {l:?}");
		assert!((r.x + l.x).abs() < 0.08, "mirrored lateral aim {r:?} {l:?}");
		Ok(())
	}

	#[test]
	fn fist_pump_clamps_beyond_one_shot_end() -> anyhow::Result<()> {
		let pump = FistPump::default();
		assert!((pump.pump_amount(1.5) - pump.pump_amount(1.0)).abs() < 1e-4);
		Ok(())
	}
}
