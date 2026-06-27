use bevy::prelude::*;

use crate::RiggedAxis;

/// Which plane a bone flexes in, relative to its [`RiggedAxis`] orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexAxis {
	/// Knee/elbow hinge (and swing-only bones): flex shares the sagittal swing axis.
	Hinge,
	/// Shoulder/pelvis lift and humerus pitch: flex in the frontal plane.
	Frontal,
}

/// Per-bone local-space swing/flex axes, derived statically from rig orientation.
///
/// These are expressed in the bone's articulation frame (its `RiggedAxis`), so the same
/// axis is valid every frame regardless of how parents move. No runtime probing or
/// world-space conversion is required.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoneArticulationFrame {
	pub swing_axis: Vec3,
	pub flex_axis: Vec3,
}

impl BoneArticulationFrame {
	pub fn new(swing_axis: Vec3, flex_axis: Vec3) -> Self {
		Self { swing_axis, flex_axis }
	}

	/// Derive swing/flex axes from a bone's [`RiggedAxis`] and anatomical flex plane.
	///
	/// Swing (forward/back) is always rotation about the bone's lateral `right` axis.
	/// Hinge flex (knee/elbow) shares that axis; frontal flex (lift/pitch) rotates about
	/// the `forward` axis. An oddly oriented bone is corrected purely by editing its
	/// `RiggedAxis` entry; the articulation axes follow automatically.
	pub fn from_rigged_axis(axis: RiggedAxis, flex: FlexAxis) -> Self {
		let swing_axis = axis.right;
		let flex_axis = match flex {
			FlexAxis::Hinge => axis.right,
			FlexAxis::Frontal => axis.forward,
		};
		Self { swing_axis, flex_axis }
	}
}

/// Compose a local bone rotation: flex then swing applied to the rest rotation.
///
/// Axes are interpreted in the bone's local articulation frame, so the result is written
/// straight back to `Transform.rotation` with no parent context.
pub fn compose_local_rotation(
	rest: Quat,
	swing_axis: Vec3,
	swing: f32,
	flex_axis: Vec3,
	flex: f32,
) -> Quat {
	let mut rotation = rest;
	if flex.abs() > f32::EPSILON {
		rotation = Quat::from_axis_angle(flex_axis, flex) * rotation;
	}
	if swing.abs() > f32::EPSILON {
		rotation = Quat::from_axis_angle(swing_axis, swing) * rotation;
	}
	rotation
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_rigged_axis_hinge_shares_swing_axis() {
		let frame = BoneArticulationFrame::from_rigged_axis(RiggedAxis::DEFAULT, FlexAxis::Hinge);
		assert_eq!(frame.swing_axis, RiggedAxis::DEFAULT.right);
		assert_eq!(frame.flex_axis, RiggedAxis::DEFAULT.right);
	}

	#[test]
	fn from_rigged_axis_frontal_uses_forward_axis() {
		let frame = BoneArticulationFrame::from_rigged_axis(RiggedAxis::DEFAULT, FlexAxis::Frontal);
		assert_eq!(frame.swing_axis, RiggedAxis::DEFAULT.right);
		assert_eq!(frame.flex_axis, RiggedAxis::DEFAULT.forward);
	}

	#[test]
	fn compose_local_rotation_applies_swing_then_flex_order() {
		let rest = Quat::from_rotation_x(0.1);
		let composed = compose_local_rotation(rest, Vec3::X, 0.3, Vec3::Z, 0.4);
		let expected =
			Quat::from_axis_angle(Vec3::X, 0.3) * Quat::from_axis_angle(Vec3::Z, 0.4) * rest;
		assert!((composed.dot(expected).abs() - 1.0).abs() < 1e-5);
	}

	#[test]
	fn compose_local_rotation_with_zero_angles_is_rest() {
		let rest = Quat::from_rotation_y(0.2);
		let composed = compose_local_rotation(rest, Vec3::X, 0.0, Vec3::Z, 0.0);
		assert!((composed.dot(rest).abs() - 1.0).abs() < 1e-6);
	}
}
