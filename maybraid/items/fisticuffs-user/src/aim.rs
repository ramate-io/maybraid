//! Body-relative jab aim from [`PlayerLook`].
//!
//! `PlayerLook` is camera −Z forward ([`player::look_forward`]). The posed
//! humanoid (`for_clip_test` / [`ArmAim`]) is character space: **+Z**
//! fight-forward, **+Y** up, **+X = character left** (the left-hand side).
//! Camera-to-character is a **180° yaw**: negate X and Z together. Flipping
//! only Z mirrors yaw.
//!
//! # Look-left (player view)
//!
//! Camera **+yaw** is looking left — the player turns their view toward the
//! left-hand side. After the 180° yaw that is character **+X**. Looking right
//! (camera −yaw) is character **−X**.
//!
//! # Cone and buckets
//!
//! | Axis | Clamp | Step |
//! |---|---|---|
//! | Pitch | −24° (belly) … +36° (above head) | 12° |
//! | Yaw | ±24° off body facing | 12° |
//!
//! Pitch buckets after clamp: −24, −12, 0, +12, +24, +36. Yaw: −24, −12, 0,
//! +12, +24. Level look maps to [`DEFAULT_JAB_TARGET`].
//!
//! # Latch
//!
//! Aim is sampled and stored when the jab **starts** (`just_started`), not at
//! Active. Chamber already points at the latched cone; look changes mid-punch
//! do not retarget.

use std::f32::consts::PI;

use bevy::prelude::Vec3;
use character_animations::animations::DEFAULT_JAB_TARGET;
use player::{look_forward, wrap_pi, yaw_xz, PlayerLook};

/// Belly: slightly below the default sternum target.
pub const PITCH_MIN: f32 = -24.0 * PI / 180.0;
/// Above the head / crown.
pub const PITCH_MAX: f32 = 36.0 * PI / 180.0;
/// Modest look-off either side of body facing.
pub const YAW_LIMIT: f32 = 24.0 * PI / 180.0;
/// Quantization step (~12°).
pub const AIM_STEP: f32 = 12.0 * PI / 180.0;

/// Reach along character +Z at level, matching [`DEFAULT_JAB_TARGET`].z.
const REACH: f32 = DEFAULT_JAB_TARGET.z;
/// Sternum height at level, matching [`DEFAULT_JAB_TARGET`].y.
const LEVEL_Y: f32 = DEFAULT_JAB_TARGET.y;

/// Quantized body-relative jab aim.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JabAim {
	/// Quantized pitch, radians. Positive looks up.
	pub pitch: f32,
	/// Quantized yaw off body facing, radians. Positive is look-left (camera +yaw).
	pub yaw: f32,
	/// Character-space point for [`Jab::target`] (+Z forward, +X character left).
	pub target: Vec3,
}

impl Default for JabAim {
	fn default() -> Self {
		Self { pitch: 0.0, yaw: 0.0, target: DEFAULT_JAB_TARGET }
	}
}

impl JabAim {
	pub fn from_look(look: &PlayerLook, body_heading: Vec3) -> Self {
		Self::from_camera_angles(look.pitch, wrap_pi(look.yaw - body_camera_yaw(body_heading)))
	}

	/// `pitch` / `relative_yaw` are camera-convention radians (see [`look_forward`]).
	pub fn from_camera_angles(pitch: f32, relative_yaw: f32) -> Self {
		let pitch = quantize_clamped(pitch, PITCH_MIN, PITCH_MAX, AIM_STEP);
		let yaw = quantize_clamped(relative_yaw, -YAW_LIMIT, YAW_LIMIT, AIM_STEP);
		Self { pitch, yaw, target: target_from_quantized(pitch, yaw) }
	}

	pub fn level() -> Self {
		Self::from_camera_angles(0.0, 0.0)
	}

	/// Character-space unit look (+X left, +Y up, +Z forward).
	pub fn direction(&self) -> Vec3 {
		let dir = Vec3::new(self.target.x, self.target.y - LEVEL_Y, self.target.z);
		if dir.length_squared() < 1e-8 {
			Vec3::Z
		} else {
			dir.normalize()
		}
	}
}

/// Camera −Z yaw of a character-space heading (+Z fight-forward).
pub fn body_camera_yaw(heading: Vec3) -> f32 {
	yaw_xz(-heading)
}

pub fn quantize_clamped(angle: f32, min: f32, max: f32, step: f32) -> f32 {
	let clamped = angle.clamp(min, max);
	if step <= 0.0 {
		return clamped;
	}
	quantize(clamped, step).clamp(min, max)
}

pub fn quantize(angle: f32, step: f32) -> f32 {
	if step <= 0.0 {
		return angle;
	}
	(angle / step).round() * step
}

/// Camera look → character space by a 180° yaw (negate X and Z).
///
/// Camera +X is the player's right; posed character +X is the player's left.
fn character_look_dir(pitch: f32, yaw: f32) -> Vec3 {
	let camera = look_forward(&PlayerLook { yaw, pitch, first_person: true, focus: 0.0 });
	Vec3::new(-camera.x, camera.y, -camera.z)
}

fn target_from_quantized(pitch: f32, yaw: f32) -> Vec3 {
	let dir = character_look_dir(pitch, yaw);
	Vec3::new(dir.x * REACH, LEVEL_Y + dir.y * REACH, dir.z * REACH)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn deg(step_count: i32) -> f32 {
		step_count as f32 * AIM_STEP
	}

	#[test]
	fn clamps_and_quantizes_pitch_to_twelve_degree_buckets() {
		let belly = JabAim::from_camera_angles(-1.0, 0.0);
		assert!((belly.pitch - PITCH_MIN).abs() < 1e-5);
		assert_eq!(belly.pitch, quantize(PITCH_MIN, AIM_STEP));

		let crown = JabAim::from_camera_angles(1.0, 0.0);
		assert!((crown.pitch - PITCH_MAX).abs() < 1e-5);

		let near = JabAim::from_camera_angles(5.0 * PI / 180.0, 0.0);
		assert!((near.pitch - 0.0).abs() < 1e-5);

		let mid = JabAim::from_camera_angles(14.0 * PI / 180.0, 0.0);
		assert!((mid.pitch - deg(1)).abs() < 1e-4);
	}

	#[test]
	fn clamps_and_quantizes_yaw_to_twelve_degree_buckets() {
		let left = JabAim::from_camera_angles(0.0, 1.0);
		assert!((left.yaw - YAW_LIMIT).abs() < 1e-5);
		let right = JabAim::from_camera_angles(0.0, -1.0);
		assert!((right.yaw + YAW_LIMIT).abs() < 1e-5);
		let snap = JabAim::from_camera_angles(0.0, 10.0 * PI / 180.0);
		assert!((snap.yaw - deg(1)).abs() < 1e-4);
	}

	#[test]
	fn level_look_is_default_jab_target() {
		let aim = JabAim::level();
		assert!((aim.target - DEFAULT_JAB_TARGET).length() < 1e-4);
		assert!(aim.target.z > 0.6);
		assert!(aim.target.x.abs() < 1e-4);
	}

	#[test]
	fn look_up_raises_target_y_look_down_lowers_it() {
		let up = JabAim::from_camera_angles(PITCH_MAX, 0.0);
		let down = JabAim::from_camera_angles(PITCH_MIN, 0.0);
		let level = JabAim::level();
		assert!(up.target.y > level.target.y + 0.15);
		assert!(down.target.y < level.target.y - 0.1);
		assert!(up.target.x.abs() < 1e-4);
		assert!(down.target.x.abs() < 1e-4);
	}

	#[test]
	fn look_left_moves_target_toward_character_plus_x() {
		// Player-view look-left is camera +yaw. Posed +X is the left-hand side.
		let left = JabAim::from_camera_angles(0.0, YAW_LIMIT);
		let right = JabAim::from_camera_angles(0.0, -YAW_LIMIT);
		assert!(left.target.x > 0.15, "look-left must be +X, got {left:?}");
		assert!(right.target.x < -0.15, "look-right must be −X, got {right:?}");
		assert!(left.direction().x > 0.0);
		assert!(right.direction().x < 0.0);
	}

	#[test]
	fn camera_to_character_is_a_180_yaw() {
		let left = character_look_dir(0.0, YAW_LIMIT);
		let camera = look_forward(&PlayerLook {
			yaw: YAW_LIMIT,
			pitch: 0.0,
			first_person: true,
			focus: 0.0,
		});
		assert!((left - Vec3::new(-camera.x, camera.y, -camera.z)).length() < 1e-5);
		assert!(left.x > 0.0, "look-left is +X after the 180° yaw, got {left:?}");
		assert!(left.z > 0.0);
	}

	#[test]
	fn body_aligned_heading_gives_zero_relative_yaw() {
		// Character +Z heading of −world Z matches look.yaw = 0 (camera −Z).
		let look = PlayerLook { yaw: 0.0, pitch: 0.0, first_person: true, focus: 0.0 };
		let aim = JabAim::from_look(&look, -Vec3::Z);
		assert!(aim.yaw.abs() < 1e-4);
		assert!((aim.target - DEFAULT_JAB_TARGET).length() < 1e-4);
	}

	#[test]
	fn look_forward_minus_z_is_respected() {
		let look = PlayerLook { yaw: 0.0, pitch: 0.0, first_person: true, focus: 0.0 };
		assert!((look_forward(&look) + Vec3::Z).length() < 1e-4);
	}
}
