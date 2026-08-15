//! Approximate terrain pitch from a rest support span. No IK.
//!
//! Sample ground at two points along facing (front / hind girdle), take
//! `atan2(Δh, span)`, and apply a skeleton-family fraction of that angle to
//! the visual. The physics capsule stays upright.

use bevy::prelude::*;

use crate::rig::RigSkeletonKind;

/// Radians of visual pitch around local X, plus the rest support span used to
/// sample terrain. Insert on the character visual; a host system writes
/// [`Self::radians`] from height samples.
#[derive(Component, Clone, Copy, Debug)]
pub struct TerrainPitch {
	pub half_span: f32,
	pub weight: f32,
	pub radians: f32,
}

impl TerrainPitch {
	pub fn new(kind: RigSkeletonKind, half_span: f32) -> Self {
		Self { half_span, weight: pitch_weight(kind), radians: 0.0 }
	}
}

pub const MAX_PITCH: f32 = 40.0_f32.to_radians();
pub const PITCH_RATE: f32 = 3.0;

const HUMANOID_HALF_SPAN: f32 = 0.22;
const QUADRUPED_HALF_SPAN: f32 = 0.9;
const FORELIMBED_HALF_SPAN: f32 = 0.4;
const MIN_MEASURED_HALF_SPAN: f32 = 0.12;

pub const QUADRUPED_FRONT: &[&str] = &["shoulder.L", "shoulder.R"];
pub const QUADRUPED_HIND: &[&str] = &["hip.L", "hip.R"];

pub fn default_half_span(kind: RigSkeletonKind) -> f32 {
	match kind {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => HUMANOID_HALF_SPAN,
		RigSkeletonKind::Quadruped => QUADRUPED_HALF_SPAN,
		RigSkeletonKind::Forelimbed => FORELIMBED_HALF_SPAN,
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

/// Rest wheelbase from girdle world positions, or the family default if the
/// bones are not ready (still stacked at the origin).
pub fn half_span_from_girdles(
	kind: RigSkeletonKind,
	front: Option<Vec3>,
	hind: Option<Vec3>,
) -> f32 {
	let fallback = default_half_span(kind);
	let (Some(front), Some(hind)) = (front, hind) else {
		return fallback;
	};
	let delta = Vec2::new(front.x - hind.x, front.z - hind.z).length() * 0.5;
	if delta < MIN_MEASURED_HALF_SPAN {
		fallback
	} else {
		delta
	}
}

/// Pitch that puts a rigid plank on the front/hind chord, clamped.
pub fn observed_pitch(front_height: f32, hind_height: f32, half_span: f32) -> f32 {
	let run = (2.0 * half_span).max(1e-3);
	((front_height - hind_height) / run).atan().clamp(-MAX_PITCH, MAX_PITCH)
}

pub fn step_toward(current: f32, target: f32, dt: f32) -> f32 {
	let delta = target - current;
	let max_step = PITCH_RATE * dt;
	current + delta.clamp(-max_step, max_step)
}

/// Yaw from flattened facing (same convention as `look_to(-facing)`), then pitch.
pub fn facing_with_pitch(facing_xz: Vec3, pitch: f32) -> Quat {
	let facing = Vec3::new(facing_xz.x, 0.0, facing_xz.z);
	if facing.length_squared() < 1e-6 {
		return Quat::from_rotation_x(pitch);
	}
	Transform::IDENTITY.looking_to(-facing, Vec3::Y).rotation * Quat::from_rotation_x(pitch)
}

/// Lift so neither girdle sits below the hip-clearance plane after pitch.
pub fn support_lift(
	hip_y: f32,
	center_height: f32,
	front_height: f32,
	hind_height: f32,
	half_span: f32,
	pitch: f32,
) -> f32 {
	let clearance = hip_y - center_height;
	let front_y = hip_y + pitch.sin() * half_span;
	let hind_y = hip_y - pitch.sin() * half_span;
	let front_err = (front_height + clearance) - front_y;
	let hind_err = (hind_height + clearance) - hind_y;
	front_err.max(hind_err).max(0.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn uphill_front_is_positive_pitch() {
		let pitch = observed_pitch(2.0, 1.0, 0.5);
		assert!(pitch > 0.0);
		assert!(pitch <= MAX_PITCH);
	}

	#[test]
	fn flat_ground_is_zero() {
		assert_eq!(observed_pitch(3.0, 3.0, 0.9), 0.0);
	}

	#[test]
	fn missing_girdles_use_family_default() {
		assert_eq!(
			half_span_from_girdles(RigSkeletonKind::Quadruped, None, None),
			QUADRUPED_HALF_SPAN
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
		assert!((half_span_from_girdles(RigSkeletonKind::Quadruped, Some(front), Some(hind)) - 1.0)
			.abs() < 1e-5);
	}

	#[test]
	fn lift_is_small_on_a_gentle_plane() {
		let half = 1.0;
		let pitch = observed_pitch(0.2, -0.2, half);
		let lift = support_lift(2.0, 0.0, 0.2, -0.2, half, pitch);
		assert!(lift < 0.05, "gentle planar slope should need little lift, got {lift}");
	}
}
