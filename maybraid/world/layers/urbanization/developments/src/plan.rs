//! Plan-view (XZ) geometry shared by development layouts and their hosts.

use std::f32::consts::TAU;

use bevy::transform::components::Transform;
use bevy_math::bounding::Aabb3d;
use bevy_math::{Quat, Vec2, Vec3};

/// Map a unit sample in \([0, 1]\) onto a heading in \([0, \tau]\).
pub fn sample_confines_yaw(unit: f32) -> f32 {
	unit.clamp(0.0, 1.0) * TAU
}

/// World-XZ AABB of a `width` × `depth` rectangle yawed about \(+Y\).
pub fn yawed_plan_aabb_extent(width: f32, depth: f32, yaw: f32) -> Vec2 {
	let (sin, cos) = yaw.sin_cos();
	let abs_c = cos.abs();
	let abs_s = sin.abs();
	Vec2::new(width * abs_c + depth * abs_s, width * abs_s + depth * abs_c)
}

/// Rotate about \(+Y\) through `center_xz` without orbiting the world origin.
///
/// Geometry is authored at world positions on the cell. Bevy applies
/// \(R p + T\), so \(T = c - R c\) keeps the cell center fixed:
/// \(R(p - c) + c\).
pub fn yaw_about_xz(center_xz: Vec2, yaw: f32) -> Transform {
	let center = Vec3::new(center_xz.x, 0.0, center_xz.y);
	let rotation = Quat::from_rotation_y(yaw);
	Transform { translation: center - rotation * center, rotation, scale: Vec3::ONE }
}

/// Per-cell salt for [`procedural_common::SeededHash`] draws.
pub fn cell_salt(cell: Aabb3d) -> u32 {
	cell.min.x.to_bits().wrapping_mul(73856093) ^ cell.min.z.to_bits().wrapping_mul(19349663)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sample_confines_yaw_covers_the_circle() {
		assert!((sample_confines_yaw(0.0) - 0.0).abs() < 1e-6);
		assert!((sample_confines_yaw(0.5) - std::f32::consts::PI).abs() < 1e-5);
		assert!((sample_confines_yaw(1.0) - TAU).abs() < 1e-5);
	}

	#[test]
	fn yaw_about_xz_keeps_the_center_fixed() {
		let center_xz = Vec2::new(250.0, -100.0);
		let yaw = std::f32::consts::FRAC_PI_4;
		let transform = yaw_about_xz(center_xz, yaw);
		let center = Vec3::new(center_xz.x, 4.0, center_xz.y);
		let stayed = transform.transform_point(center);
		assert!((stayed - center).length() < 1e-4);
		let offset = Vec3::new(center_xz.x + 10.0, 4.0, center_xz.y);
		let rotated = transform.transform_point(offset);
		let expected = center + Quat::from_rotation_y(yaw) * (offset - center);
		assert!((rotated - expected).length() < 1e-4);
	}
}
