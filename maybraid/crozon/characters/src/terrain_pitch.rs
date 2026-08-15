//! Approximate terrain pitch and roll from a rest support span. No IK.
//!
//! Sample ground at front / hind / left / right, take `atan(Δh / run)`, and
//! apply a skeleton-family fraction to the visual. Mesh faces `+Z`; positive
//! local `X` dips the nose, so sagittal slope is negated. The capsule stays
//! upright.

use bevy::prelude::*;

use crate::rig::RigSkeletonKind;

/// Visual tilt plus the rest support span used to sample terrain.
#[derive(Component, Clone, Copy, Debug)]
pub struct TerrainPitch {
	pub half_span: f32,
	pub half_width: f32,
	pub weight: f32,
	/// Local-X radians (nose down is positive).
	pub pitch: f32,
	/// Local-Z radians (right side up is positive).
	pub roll: f32,
}

impl TerrainPitch {
	pub fn new(kind: RigSkeletonKind, half_span: f32, half_width: f32) -> Self {
		Self { half_span, half_width, weight: pitch_weight(kind), pitch: 0.0, roll: 0.0 }
	}
}

pub const MAX_TILT: f32 = 40.0_f32.to_radians();
pub const TILT_RATE: f32 = 3.0;

const HUMANOID_HALF_SPAN: f32 = 0.22;
const QUADRUPED_HALF_SPAN: f32 = 0.9;
const FORELIMBED_HALF_SPAN: f32 = 0.4;
const HUMANOID_HALF_WIDTH: f32 = 0.18;
const QUADRUPED_HALF_WIDTH: f32 = 0.45;
const FORELIMBED_HALF_WIDTH: f32 = 0.25;
const MIN_MEASURED: f32 = 0.12;

pub const QUADRUPED_FRONT: &[&str] = &["shoulder.L", "shoulder.R"];
pub const QUADRUPED_HIND: &[&str] = &["hip.L", "hip.R"];
pub const QUADRUPED_LEFT: &[&str] = &["shoulder.L", "hip.L"];
pub const QUADRUPED_RIGHT: &[&str] = &["shoulder.R", "hip.R"];

pub fn default_half_span(kind: RigSkeletonKind) -> f32 {
	match kind {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => HUMANOID_HALF_SPAN,
		RigSkeletonKind::Quadruped => QUADRUPED_HALF_SPAN,
		RigSkeletonKind::Forelimbed => FORELIMBED_HALF_SPAN,
	}
}

pub fn default_half_width(kind: RigSkeletonKind) -> f32 {
	match kind {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => HUMANOID_HALF_WIDTH,
		RigSkeletonKind::Quadruped => QUADRUPED_HALF_WIDTH,
		RigSkeletonKind::Forelimbed => FORELIMBED_HALF_WIDTH,
	}
}

/// How much of the observed slope to apply. Long bodies need more or they sink.
pub fn pitch_weight(kind: RigSkeletonKind) -> f32 {
	match kind {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => 0.4,
		RigSkeletonKind::Quadruped => 0.9,
		RigSkeletonKind::Forelimbed => 0.7,
	}
}

/// Midpoint of named bones in XZ, if at least one exists.
pub fn girdle_midpoint(positions: impl IntoIterator<Item = Vec3>) -> Option<Vec3> {
	let mut sum = Vec3::ZERO;
	let mut n = 0u32;
	for p in positions {
		sum += p;
		n += 1;
	}
	(n > 0).then(|| sum / n as f32)
}

fn measured_half(a: Option<Vec3>, b: Option<Vec3>, fallback: f32) -> f32 {
	let (Some(a), Some(b)) = (a, b) else {
		return fallback;
	};
	let delta = Vec2::new(a.x - b.x, a.z - b.z).length() * 0.5;
	if delta < MIN_MEASURED {
		fallback
	} else {
		delta
	}
}

/// Rest wheelbase from girdle world positions, or the family default.
pub fn half_span_from_girdles(
	kind: RigSkeletonKind,
	front: Option<Vec3>,
	hind: Option<Vec3>,
) -> f32 {
	measured_half(front, hind, default_half_span(kind))
}

/// Rest stance width from left/right world positions, or the family default.
pub fn half_width_from_sides(kind: RigSkeletonKind, left: Option<Vec3>, right: Option<Vec3>) -> f32 {
	measured_half(left, right, default_half_width(kind))
}

fn slope_angle(high_side: f32, low_side: f32, half_run: f32) -> f32 {
	let run = (2.0 * half_run).max(1e-3);
	((high_side - low_side) / run).atan().clamp(-MAX_TILT, MAX_TILT)
}

/// Local-X angle: nose up when the front sample is higher (`+Z` mesh).
pub fn observed_pitch(front_height: f32, hind_height: f32, half_span: f32) -> f32 {
	-slope_angle(front_height, hind_height, half_span)
}

/// Local-Z angle: right side up when the right sample is higher.
pub fn observed_roll(left_height: f32, right_height: f32, half_width: f32) -> f32 {
	slope_angle(right_height, left_height, half_width)
}

pub fn step_toward(current: f32, target: f32, dt: f32) -> f32 {
	let delta = target - current;
	let max_step = TILT_RATE * dt;
	current + delta.clamp(-max_step, max_step)
}

/// Yaw from flattened facing (`look_to(-facing)`), then local pitch and roll.
pub fn facing_with_tilt(facing_xz: Vec3, pitch: f32, roll: f32) -> Quat {
	let facing = Vec3::new(facing_xz.x, 0.0, facing_xz.z);
	let yaw = if facing.length_squared() < 1e-6 {
		Quat::IDENTITY
	} else {
		Transform::IDENTITY.looking_to(-facing, Vec3::Y).rotation
	};
	yaw * Quat::from_rotation_x(pitch) * Quat::from_rotation_z(roll)
}

/// Lift so support samples stay on the hip-clearance plane after tilt.
pub fn support_lift(
	hip_y: f32,
	center_height: f32,
	front_height: f32,
	hind_height: f32,
	left_height: f32,
	right_height: f32,
	half_span: f32,
	half_width: f32,
	pitch: f32,
	roll: f32,
) -> f32 {
	let clearance = hip_y - center_height;
	// +Rx dips +Z (front); +Rz raises +X (right).
	let front_y = hip_y - pitch.sin() * half_span;
	let hind_y = hip_y + pitch.sin() * half_span;
	let left_y = hip_y - roll.sin() * half_width;
	let right_y = hip_y + roll.sin() * half_width;
	let err = |sample, y| (sample + clearance) - y;
	err(front_height, front_y)
		.max(err(hind_height, hind_y))
		.max(err(left_height, left_y))
		.max(err(right_height, right_y))
		.max(0.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn uphill_front_pitches_nose_up() {
		let pitch = observed_pitch(2.0, 1.0, 0.5);
		assert!(pitch < 0.0);
		assert!(pitch.abs() <= MAX_TILT);
		let nose = facing_with_tilt(Vec3::Z, pitch, 0.0) * Vec3::Z;
		assert!(nose.y > 0.0, "uphill should raise mesh +Z, y={}", nose.y);
	}

	#[test]
	fn high_right_rolls_right_up() {
		let roll = observed_roll(1.0, 2.0, 0.5);
		assert!(roll > 0.0);
		let right = facing_with_tilt(Vec3::Z, 0.0, roll) * Vec3::X;
		assert!(right.y > 0.0, "high right should raise mesh +X, y={}", right.y);
	}

	#[test]
	fn flat_ground_is_zero() {
		assert_eq!(observed_pitch(3.0, 3.0, 0.9), 0.0);
		assert_eq!(observed_roll(3.0, 3.0, 0.45), 0.0);
	}

	#[test]
	fn missing_girdles_use_family_default() {
		assert_eq!(
			half_span_from_girdles(RigSkeletonKind::Quadruped, None, None),
			QUADRUPED_HALF_SPAN
		);
		assert_eq!(
			half_width_from_sides(RigSkeletonKind::Quadruped, None, None),
			QUADRUPED_HALF_WIDTH
		);
	}

	#[test]
	fn stacked_girdles_use_family_default() {
		let origin = Vec3::new(10.0, 1.0, 4.0);
		assert_eq!(
			half_span_from_girdles(RigSkeletonKind::Quadruped, Some(origin), Some(origin)),
			QUADRUPED_HALF_SPAN
		);
	}

	#[test]
	fn measured_girdles_win_when_separated() {
		let front = Vec3::new(0.0, 1.0, 2.0);
		let hind = Vec3::new(0.0, 1.0, 0.0);
		assert!(
			(half_span_from_girdles(RigSkeletonKind::Quadruped, Some(front), Some(hind)) - 1.0)
				.abs() < 1e-5
		);
	}

	#[test]
	fn lift_is_small_on_a_gentle_plane() {
		let half = 1.0;
		let pitch = observed_pitch(0.2, -0.2, half);
		let lift = support_lift(2.0, 0.0, 0.2, -0.2, 0.0, 0.0, half, 0.45, pitch, 0.0);
		assert!(lift < 0.05, "gentle planar slope should need little lift, got {lift}");
	}
}
