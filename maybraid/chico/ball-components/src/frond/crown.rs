//! Crown-level frond direction caps (palm / fern clusters).

use bevy::prelude::*;

const GOLDEN_ANGLE: f32 = 2.399_963_229_728_653_32;

/// Outward-and-downward frond headings for palm-like crowns.
pub(crate) fn crown_directions(
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
			let azimuth = GOLDEN_ANGLE.mul_add(fi, phase);
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

/// Per-frond length multiplier in `[min, max]` (deterministic from seed).
pub(crate) fn length_scale(index: u32, seed: i32, min: f32, max: f32) -> f32 {
	let t = ((seed.wrapping_add(index as i32) as f32) * 0.47).sin().abs();
	min + (max - min) * t
}

/// Align frond-local +X to the crown emission direction.
pub(crate) fn align_frond_direction(direction: Vec3) -> Quat {
	let axis = Vec3::X;
	let d = direction.normalize_or_zero();
	if d.length_squared() < 1e-12 {
		return Quat::IDENTITY;
	}
	let dot = axis.dot(d);
	if dot > 1.0 - 1e-5 {
		return Quat::IDENTITY;
	}
	if dot < -1.0 + 1e-5 {
		return Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI);
	}
	Quat::from_rotation_arc(axis, d)
}
