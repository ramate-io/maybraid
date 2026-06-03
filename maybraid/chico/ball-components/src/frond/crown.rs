//! Crown-level frond direction caps (palm / fern clusters).

use bevy::prelude::*;

const GOLDEN_ANGLE: f32 = 2.399_963_229_728_653_32;

/// Outward frond headings: horizontal fan with separate pitch (lift / droop) and azimuth spread.
///
/// `outward_spread_radians` wobbles azimuth only; it is not added into the downward pitch (avoids
/// the “flanged” look where high spread reads as near-horizontal emission plus steep local droop).
pub(crate) fn crown_directions(
	count: u32,
	seed: i32,
	downward_tilt_radians: f32,
	outward_spread_radians: f32,
	emission_lift_radians: f32,
) -> Vec<Vec3> {
	let n = count.max(1);
	let phase = (seed as f32).mul_add(0.271, 0.0);
	(0..n)
		.map(|i| {
			let fi = i as f32;
			let down = downward_tilt_radians
				* (0.65 + 0.35 * ((seed.wrapping_add(i as i32) as f32) * 0.19).cos().abs());
			let spread = outward_spread_radians
				* (0.4 + 0.6 * ((seed.wrapping_add(i as i32) as f32) * 0.23).sin().abs());
			let lift = emission_lift_radians
				* (0.7 + 0.3 * ((seed.wrapping_add(i as i32) as f32) * 0.31).cos().abs());
			let pitch = lift - down;
			let azimuth = GOLDEN_ANGLE.mul_add(fi, phase)
				+ spread * ((seed.wrapping_add(i as i32) as f32) * 0.41).sin();
			Vec3::new(
				pitch.cos() * azimuth.cos(),
				pitch.sin(),
				pitch.cos() * azimuth.sin(),
			)
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
pub fn align_frond_direction(direction: Vec3) -> Quat {
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn spread_does_not_flatten_emission_to_pure_horizontal() {
		let steep = crown_directions(1, 0, 0.8, 0.0, 0.0)[0];
		let with_spread = crown_directions(1, 0, 0.8, 1.2, 0.0)[0];
		assert!(steep.y < -0.5, "expected downward pitch: {steep:?}");
		assert!(
			with_spread.y < -0.4,
			"spread should not cancel downward pitch: {with_spread:?}"
		);
	}

	#[test]
	fn emission_lift_can_point_outward_and_up() {
		let d = crown_directions(1, 0, 0.15, 0.5, 0.35)[0];
		assert!(d.y > 0.1, "expected lifted emission: {d:?}");
	}
}
