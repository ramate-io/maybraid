//! Respectful forward bow: dip the torso, soften the knees, and let both arms hang.

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

pub const DIP_END: f32 = 0.24;
pub const HOLD_END: f32 = 0.62;
const SPINE_PITCH: f32 = 0.50;
const NECK_NOD: f32 = 0.20;
const ELBOW_FLEX: f32 = 0.38;
const HUMERUS_DOWN: f32 = 0.72;
const HUMERUS_FORWARD: f32 = 0.28;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bow;

impl Bow {
	pub fn bow_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < DIP_END {
			smoothstep(t / DIP_END)
		} else if t < HOLD_END {
			1.0
		} else {
			1.0 - smoothstep((t - HOLD_END) / (1.0 - HOLD_END))
		}
	}

	pub fn spine_pitch(&self, _progress: f32) -> f32 {
		SPINE_PITCH
	}

	pub fn neck_nod(&self, _progress: f32) -> f32 {
		NECK_NOD
	}

	pub fn elbow_flex(&self, _progress: f32) -> f32 {
		ELBOW_FLEX
	}

	/// Character-space target for each humerus; the adapter mirrors its small
	/// lateral component with `Side::sign()` before resolving the pose.
	pub fn humerus_along(&self, side: Side) -> Vec3 {
		Vec3::new(side.sign() * 0.12, -HUMERUS_DOWN, HUMERUS_FORWARD).normalize()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn bow_clamps_and_eases_between_neutral_endpoints() -> anyhow::Result<()> {
		let bow = Bow;
		assert_eq!(bow.bow_amount(-0.5), bow.bow_amount(0.0));
		assert_eq!(bow.bow_amount(1.5), bow.bow_amount(1.0));
		assert!(bow.bow_amount(0.0).abs() < 1e-6);
		assert!(bow.bow_amount(1.0).abs() < 1e-6);
		assert!(bow.bow_amount(DIP_END) > 0.99);
		assert!(bow.bow_amount(HOLD_END) > 0.99);
		assert!(bow.bow_amount(0.5) > 0.99);
		assert!(bow.bow_amount(0.75) < bow.bow_amount(HOLD_END));
		Ok(())
	}
}
