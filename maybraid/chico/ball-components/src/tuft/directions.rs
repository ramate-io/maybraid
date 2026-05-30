//! Golden-angle direction caps for tuft element placement.

use bevy::prelude::*;

/// Even azimuth spacing on a tuft hemisphere or droop cap.
pub(crate) struct CapDirections;

impl CapDirections {
	const GOLDEN_ANGLE: f32 = 2.399_963_229_728_653_32;

	/// Upward-biased directions on a spherical cap (shared anchor, mostly +Y).
	pub(crate) fn upward(count: u32, seed: i32, max_tilt_radians: f32) -> Vec<Vec3> {
		let n = count.max(1);
		let phase = (seed as f32).mul_add(0.173, 0.0);
		(0..n)
			.map(|i| {
				let fi = i as f32;
				let azimuth = Self::GOLDEN_ANGLE.mul_add(fi, phase);
				let tilt = max_tilt_radians
					* (0.55 + 0.45 * ((seed.wrapping_add(i as i32) as f32) * 0.31).sin().abs());
				Vec3::new(tilt.sin() * azimuth.cos(), tilt.cos(), tilt.sin() * azimuth.sin())
					.normalize_or_zero()
			})
			.collect()
	}

	/// Downward-and-outward directions for drooping strands (negative Y bias).
	pub(crate) fn weeping(
		count: u32,
		seed: i32,
		downward_tilt_radians: f32,
		outward_spread_radians: f32,
	) -> Vec<Vec3> {
		let n = count.max(1);
		let phase = (seed as f32).mul_add(0.271, 0.0);
		(0..n)
			.map(|i| {
				let fi = i as f32;
				let azimuth = Self::GOLDEN_ANGLE.mul_add(fi, phase);
				let down = downward_tilt_radians
					* (0.65 + 0.35 * ((seed.wrapping_add(i as i32) as f32) * 0.19).cos().abs());
				let spread = outward_spread_radians
					* (0.4 + 0.6 * ((seed.wrapping_add(i as i32) as f32) * 0.23).sin().abs());
				let tilt = (down + spread).min(std::f32::consts::FRAC_PI_2 - 0.05);
				Vec3::new(tilt.sin() * azimuth.cos(), -tilt.cos(), tilt.sin() * azimuth.sin())
					.normalize_or_zero()
			})
			.collect()
	}

	/// Per-element length multiplier in `[min, max]` (deterministic from seed).
	pub(crate) fn length_scale(index: u32, seed: i32, min: f32, max: f32) -> f32 {
		let t = ((seed.wrapping_add(index as i32) as f32) * 0.47).sin().abs();
		min + (max - min) * t
	}
}
