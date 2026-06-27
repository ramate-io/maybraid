use bevy::prelude::*;

use crate::RiggedAxis;

/// Compose a local bone rotation: flex then swing applied to the rest rotation.
///
/// Axes come directly from the bone's [`RiggedAxis`], so the result is written straight
/// back to `Transform.rotation` with no parent context or runtime probing.
pub fn compose_local_rotation(rest: Quat, axis: RiggedAxis, swing: f32, flex: f32) -> Quat {
	let mut rotation = rest;
	if flex.abs() > f32::EPSILON {
		rotation = Quat::from_axis_angle(axis.flex_axis, flex) * rotation;
	}
	if swing.abs() > f32::EPSILON {
		rotation = Quat::from_axis_angle(axis.swing_axis, swing) * rotation;
	}
	rotation
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compose_local_rotation_applies_swing_then_flex_order() {
		let rest = Quat::from_rotation_x(0.1);
		let axis = RiggedAxis { swing_axis: Vec3::X, flex_axis: Vec3::Z, twist_axis: Vec3::Y };
		let composed = compose_local_rotation(rest, axis, 0.3, 0.4);
		let expected =
			Quat::from_axis_angle(Vec3::X, 0.3) * Quat::from_axis_angle(Vec3::Z, 0.4) * rest;
		assert!((composed.dot(expected).abs() - 1.0).abs() < 1e-5);
	}

	#[test]
	fn compose_local_rotation_with_zero_angles_is_rest() {
		let rest = Quat::from_rotation_y(0.2);
		let composed = compose_local_rotation(rest, RiggedAxis::DEFAULT, 0.0, 0.0);
		assert!((composed.dot(rest).abs() - 1.0).abs() < 1e-6);
	}
}
