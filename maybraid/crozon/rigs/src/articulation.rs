use bevy::prelude::*;

pub const WORLD_FORWARD: Vec3 = Vec3::NEG_Z;
pub const WORLD_LATERAL: Vec3 = Vec3::X;

/// Per-bone world-space swing/flex axes discovered from rest geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoneArticulationFrame {
	pub swing_axis: Vec3,
	pub flex_axis: Vec3,
	/// +1 or −1 so mirrored forearms flex forward consistently.
	pub flex_sign: f32,
}

impl BoneArticulationFrame {
	pub fn new(swing_axis: Vec3, flex_axis: Vec3, flex_sign: f32) -> Self {
		Self { swing_axis, flex_axis, flex_sign }
	}
}

/// Apply swing/flex in world space, then convert back to local bone rotation.
pub fn compose_world_rotations(
	rest: Quat,
	parent_rot: Quat,
	swing_axis: Vec3,
	swing: f32,
	flex_axis: Vec3,
	flex: f32,
) -> Quat {
	let mut global = parent_rot * rest;
	if flex.abs() > f32::EPSILON {
		global = Quat::from_axis_angle(flex_axis, flex) * global;
	}
	if swing.abs() > f32::EPSILON {
		global = Quat::from_axis_angle(swing_axis, swing) * global;
	}
	parent_rot.inverse() * global
}

/// Pick swing/flex world axes for a named animation bone.
pub fn bone_axes(bone: &str, bone_dir: Vec3) -> (Vec3, Vec3) {
	let sagittal = sagittal_world_axis(bone_dir);
	match bone {
		"forearm.L" | "forearm.R" | "shin.L" | "shin.R" => (sagittal, sagittal),
		"humerus.L" | "humerus.R" => (sagittal, pitch_down_axis(bone_dir)),
		"shoulder.L" | "shoulder.R" | "pelvis.L" | "pelvis.R" => {
			(sagittal, vertical_lift_axis(bone_dir))
		}
		_ => (sagittal, hinge_axis(bone_dir, sagittal)),
	}
}

/// World-space direction from bone origin toward its first non-zero child offset.
pub fn bone_world_direction(world_rot: Quat, child_local: Option<Vec3>) -> Vec3 {
	if let Some(local) = child_local {
		if local.length_squared() > f32::EPSILON {
			return (world_rot * local).normalize();
		}
	}

	(world_rot * Vec3::Y).normalize_or(Vec3::NEG_Y)
}

pub fn forward_flex_sign(bone_dir: Vec3, axis: Vec3) -> f32 {
	const TEST: f32 = 0.12;
	let neg = Quat::from_axis_angle(axis, -TEST) * bone_dir;
	let pos = Quat::from_axis_angle(axis, TEST) * bone_dir;
	let neg_forward = (neg - bone_dir).dot(WORLD_FORWARD);
	let pos_forward = (pos - bone_dir).dot(WORLD_FORWARD);
	if neg_forward < pos_forward {
		-1.0
	} else {
		1.0
	}
}

/// Pick the world axis whose small rotation moves the bone forward/back, not side-to-side.
pub fn sagittal_world_axis(bone_dir: Vec3) -> Vec3 {
	const CANDIDATES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];
	const TEST_ANGLE: f32 = 0.1;

	let mut best_axis = Vec3::X;
	let mut best_score = f32::NEG_INFINITY;

	for axis in CANDIDATES {
		if axis.cross(bone_dir).length_squared() < f32::EPSILON {
			continue;
		}

		let rotated = Quat::from_axis_angle(axis, TEST_ANGLE) * bone_dir;
		let delta = rotated - bone_dir;
		let forward = delta.dot(WORLD_FORWARD).abs();
		let lateral = delta.dot(WORLD_LATERAL).abs();
		let score = forward / (lateral + 1e-3);

		if score > best_score {
			best_score = score;
			best_axis = axis;
		}
	}

	best_axis
}

/// Axis that lifts or drops the bone mostly along world up.
pub fn vertical_lift_axis(bone_dir: Vec3) -> Vec3 {
	const CANDIDATES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];
	const TEST_ANGLE: f32 = 0.1;

	let mut best_axis = Vec3::Z;
	let mut best_lift = 0.0_f32;

	for axis in CANDIDATES {
		if axis.cross(bone_dir).length_squared() < f32::EPSILON {
			continue;
		}

		let rotated = Quat::from_axis_angle(axis, TEST_ANGLE) * bone_dir;
		let lift = (rotated.y - bone_dir.y).abs();
		if lift > best_lift {
			best_lift = lift;
			best_axis = axis;
		}
	}

	best_axis
}

/// Axis that pitches a T-pose arm toward world down.
pub fn pitch_down_axis(bone_dir: Vec3) -> Vec3 {
	const CANDIDATES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];
	const TEST_ANGLE: f32 = 0.1;

	let mut best_axis = Vec3::Z;
	let mut best_down = f32::NEG_INFINITY;

	for axis in CANDIDATES {
		if axis.cross(bone_dir).length_squared() < f32::EPSILON {
			continue;
		}

		let rotated = Quat::from_axis_angle(axis, TEST_ANGLE) * bone_dir;
		let downward = rotated.y - bone_dir.y;
		if downward < best_down {
			best_down = downward;
			best_axis = axis;
		}
	}

	best_axis
}

pub fn hinge_axis(bone_dir: Vec3, swing_axis: Vec3) -> Vec3 {
	bone_dir.cross(swing_axis).normalize_or(Vec3::Y)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compose_world_rotations_applies_flex_before_swing_with_identity_parent() {
		let rest = Quat::from_rotation_x(0.2);
		let swing_axis = Vec3::Y;
		let flex_axis = Vec3::X;
		let swing = 0.3;
		let flex = 0.4;

		let composed =
			compose_world_rotations(rest, Quat::IDENTITY, swing_axis, swing, flex_axis, flex);
		let expected = Quat::from_axis_angle(swing_axis, swing)
			* Quat::from_axis_angle(flex_axis, flex)
			* rest;

		let dot = composed.dot(expected).abs();
		assert!((dot - 1.0).abs() < 1e-5, "expected aligned quaternions, dot={dot}");
	}

	#[test]
	fn bone_axes_maps_shin_to_sagittal_hinge() {
		let bone_dir = Vec3::NEG_Y;
		let (swing, flex) = bone_axes("shin.L", bone_dir);
		assert_eq!(swing, flex);
	}

	#[test]
	fn bone_axes_maps_humerus_to_sagittal_and_pitch_down() {
		let bone_dir = Vec3::X;
		let (swing, flex) = bone_axes("humerus.R", bone_dir);
		assert_ne!(swing, flex);
	}

	#[test]
	fn bone_axes_maps_pelvis_to_sagittal_and_vertical_lift() {
		let bone_dir = Vec3::X;
		let (swing, flex) = bone_axes("pelvis.L", bone_dir);
		assert_ne!(swing, flex);
	}
}
