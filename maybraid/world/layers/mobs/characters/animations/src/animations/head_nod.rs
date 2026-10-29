//! Affirmative double head nod with a light upper-spine follow.

use crate::Progress;

const FIRST_NOD_CENTER: f32 = 0.24;
const SECOND_NOD_CENTER: f32 = 0.54;
const NOD_HALF_WIDTH: f32 = 0.10;
const NOD_DEPTH: f32 = 0.26;
const SPINE_FOLLOW: f32 = 0.04;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeadNod;

impl HeadNod {
	fn nod_pulse(t: f32, center: f32, half_width: f32) -> f32 {
		if half_width <= 0.0 {
			return 0.0;
		}
		let x = ((t - center) / half_width).abs();
		if x >= 1.0 {
			0.0
		} else {
			(1.0 - x).powi(2)
		}
	}

	pub fn nod_twist(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		let first = Self::nod_pulse(t, FIRST_NOD_CENTER, NOD_HALF_WIDTH);
		let second = Self::nod_pulse(t, SECOND_NOD_CENTER, NOD_HALF_WIDTH);
		NOD_DEPTH * (first + second)
	}

	pub fn spine_follow(&self, progress: f32) -> f32 {
		SPINE_FOLLOW * (self.nod_twist(progress) / NOD_DEPTH)
	}

	pub fn gesture_amount(&self, progress: f32) -> f32 {
		(self.nod_twist(progress) / NOD_DEPTH).clamp(0.0, 1.0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn head_nod_clamps_and_has_two_separate_peaks() -> anyhow::Result<()> {
		let nod = HeadNod;
		assert_eq!(nod.nod_twist(-1.0), nod.nod_twist(0.0));
		assert_eq!(nod.nod_twist(2.0), nod.nod_twist(1.0));
		assert!(nod.nod_twist(0.0).abs() < 1e-6);
		assert!(nod.nod_twist(1.0).abs() < 1e-6);
		assert!(nod.nod_twist(FIRST_NOD_CENTER) > NOD_DEPTH * 0.99);
		assert!(nod.nod_twist(SECOND_NOD_CENTER) > NOD_DEPTH * 0.99);
		assert!(nod.nod_twist(0.39) < NOD_DEPTH * 0.02);
		assert!(nod.gesture_amount(0.39) < 0.02);
		Ok(())
	}
}
