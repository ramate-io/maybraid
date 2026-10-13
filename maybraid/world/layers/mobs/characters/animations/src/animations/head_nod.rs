//! Affirmative double head nod: quick chin dips on the neck stack, arms at rest.
//!
//! Bilateral silhouette distinct from bow, shrug, salute, and wave clips.
//! Character space is +X right, +Y up, +Z fight-forward. Progress is a one-shot in `[0, 1]`.

use crate::Progress;

const FIRST_NOD_CENTER: f32 = 0.24;
const SECOND_NOD_CENTER: f32 = 0.54;
const NOD_HALF_WIDTH: f32 = 0.10;

/// Peak chin nod per dip (radians on the neck twist / nod channel).
const NOD_DEPTH: f32 = 0.26;
/// Light upper-spine follow so the read is not a rigid neck hinge.
const SPINE_FOLLOW: f32 = 0.04;

/// One-shot head-nod knobs. The rig resolver maps these onto V0 channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeadNod;

impl HeadNod {
	/// Smooth parabolic pulse centered at `center` with support `2 * half_width`.
	fn nod_pulse(t: f32, center: f32, half_width: f32) -> f32 {
		if half_width <= 0.0 {
			return 0.0;
		}
		let x = ((t - center) / half_width).abs();
		if x >= 1.0 {
			0.0
		} else {
			let w = 1.0 - x;
			w * w
		}
	}

	/// Combined chin nod for both affirmative dips. Zero at progress `0` and `1`.
	pub fn nod_twist(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		let first = Self::nod_pulse(t, FIRST_NOD_CENTER, NOD_HALF_WIDTH);
		let second = Self::nod_pulse(t, SECOND_NOD_CENTER, NOD_HALF_WIDTH);
		NOD_DEPTH * (first + second)
	}

	pub fn spine_follow(&self, progress: f32) -> f32 {
		let nod = self.nod_twist(progress);
		if nod <= 0.0 {
			0.0
		} else {
			SPINE_FOLLOW * (nod / NOD_DEPTH)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn head_nod_starts_and_ends_at_rest() -> anyhow::Result<()> {
		let nod = HeadNod;
		assert!(nod.nod_twist(0.0) < 1e-4);
		assert!(nod.nod_twist(1.0) < 1e-4);
		assert!(nod.spine_follow(0.0) < 1e-4);
		assert!(nod.spine_follow(1.0) < 1e-4);
		Ok(())
	}

	#[test]
	fn head_nod_has_two_distinct_peaks() -> anyhow::Result<()> {
		let nod = HeadNod;
		let first = nod.nod_twist(FIRST_NOD_CENTER);
		let second = nod.nod_twist(SECOND_NOD_CENTER);
		let valley = nod.nod_twist((FIRST_NOD_CENTER + SECOND_NOD_CENTER) * 0.5);
		assert!(first > NOD_DEPTH * 0.85, "first dip {first}");
		assert!(second > NOD_DEPTH * 0.85, "second dip {second}");
		assert!(valley < NOD_DEPTH * 0.35, "valley between dips {valley}");
		Ok(())
	}
}
