use bevy::prelude::*;

use crate::RiggedAxis;

/// Compose a local bone rotation: flex, twist, then swing applied to the rest rotation.
///
/// Axes come directly from the bone's [`RiggedAxis`], so the result is written straight
/// back to `Transform.rotation` with no parent context or runtime probing.
pub fn compose_local_rotation(
	rest: Quat,
	axis: RiggedAxis,
	swing: f32,
	flex: f32,
	twist: f32,
) -> Quat {
	let mut rotation = rest;
	if flex.abs() > f32::EPSILON {
		rotation = Quat::from_axis_angle(axis.flex_axis, flex) * rotation;
	}
	if twist.abs() > f32::EPSILON {
		rotation = Quat::from_axis_angle(axis.twist_axis, twist) * rotation;
	}
	if swing.abs() > f32::EPSILON {
		rotation = Quat::from_axis_angle(axis.swing_axis, swing) * rotation;
	}
	rotation
}

/// Convert a world-space displacement into a bone's parent-local space.
///
/// Bevy uses +Y up and −Z forward (glTF +Z forward is flipped on import). Pass
/// [`Vec3::NEG_Y`] scaled by drop magnitude for a world-vertical shift.
pub fn world_displacement_in_parent(parent_world: Quat, world_delta: Vec3) -> Vec3 {
	parent_world.inverse() * world_delta
}

/// Parent-local translation delta that shifts a joint without shortening its bind segment.
///
/// Bones with a non-zero rest segment (e.g. femur `translation.y = 0.25`) should not
/// receive displacement along that segment axis — doing so compresses the bone in the
/// skinned mesh. Joint bones (zero segment) take the full parent-local displacement.
pub fn axis_aware_translation_delta(
	segment_offset: Vec3,
	_axis: RiggedAxis,
	world_delta: Vec3,
	parent_world: Quat,
) -> Vec3 {
	let parent_local = world_displacement_in_parent(parent_world, world_delta);

	if segment_offset.length_squared() <= f32::EPSILON {
		return parent_local;
	}

	let segment_dir = segment_offset.normalize();
	parent_local - segment_dir * parent_local.dot(segment_dir)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::f32::consts::FRAC_PI_2;

	#[test]
	fn compose_local_rotation_applies_swing_then_flex_order() {
		let rest = Quat::from_rotation_x(0.1);
		let axis = RiggedAxis { swing_axis: Vec3::X, flex_axis: Vec3::Z, twist_axis: Vec3::Y };
		let composed = compose_local_rotation(rest, axis, 0.3, 0.4, 0.0);
		let expected = Quat::from_axis_angle(Vec3::X, 0.3)
			* Quat::from_axis_angle(Vec3::Z, 0.4)
			* rest;
		assert!((composed.dot(expected).abs() - 1.0).abs() < 1e-5);
	}

	#[test]
	fn compose_local_rotation_applies_flex_twist_swing_order() {
		let rest = Quat::IDENTITY;
		let axis = RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::X, twist_axis: Vec3::Z };
		let composed = compose_local_rotation(rest, axis, 0.2, 0.3, 0.4);
		let expected = Quat::from_axis_angle(Vec3::Y, 0.2)
			* Quat::from_axis_angle(Vec3::Z, 0.4)
			* Quat::from_axis_angle(Vec3::X, 0.3);
		assert!((composed.dot(expected).abs() - 1.0).abs() < 1e-5);
	}

	#[test]
	fn compose_local_rotation_with_zero_angles_is_rest() {
		let rest = Quat::from_rotation_y(0.2);
		let composed = compose_local_rotation(rest, RiggedAxis::DEFAULT, 0.0, 0.0, 0.0);
		assert!((composed.dot(rest).abs() - 1.0).abs() < 1e-6);
	}

	#[test]
	fn axis_aware_delta_preserves_segment_length_on_y_bone() {
		let segment = Vec3::new(0.0, 0.25, 0.0);
		let drop = Vec3::new(0.0, -0.15, 0.0);
		let delta = axis_aware_translation_delta(segment, RiggedAxis::DEFAULT, drop, Quat::IDENTITY);
		assert_eq!(delta, Vec3::ZERO);
	}

	#[test]
	fn axis_aware_delta_shifts_joint_bone_on_world_y() {
		let drop = Vec3::new(0.0, -0.15, 0.0);
		let delta =
			axis_aware_translation_delta(Vec3::ZERO, RiggedAxis::DEFAULT, drop, Quat::IDENTITY);
		assert_eq!(delta, drop);
	}

	#[test]
	fn axis_aware_delta_converts_world_y_through_parent_rotation() {
		let parent = Quat::from_euler(EulerRot::XYZ, -FRAC_PI_2, 0.0, -FRAC_PI_2);
		let drop = Vec3::new(0.0, -0.15, 0.0);
		let delta =
			axis_aware_translation_delta(Vec3::ZERO, RiggedAxis::DEFAULT, drop, parent);
		assert!(delta.length() > 0.0);
		assert!((delta.y).abs() < 0.01, "pelvis-local drop should not stay on parent Y");
	}
}
