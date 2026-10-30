//! Shared character locomotion limits used by movement and spawn support.

use bevy::prelude::*;

/// Grounded-walk feel. The uphill limit is also the steepest surface that
/// can accept a pending arrival. Downhill and contour movement stay unrestricted.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CharacterLocomotion {
	pub max_slope_angle: f32,
}

impl Default for CharacterLocomotion {
	fn default() -> Self {
		Self { max_slope_angle: std::f32::consts::PI * 0.45 }
	}
}

impl CharacterLocomotion {
	/// A finite, upward-facing normal inside the configured uphill angle.
	/// Side walls and downward faces cannot become spawn support, even if a
	/// caller configures a movement angle greater than a right angle.
	pub fn accepts_support_normal(self, normal: Vec3) -> bool {
		if !normal.is_finite() || !self.max_slope_angle.is_finite() || self.max_slope_angle < 0.0 {
			return false;
		}
		let Some(normal) = normal.try_normalize() else { return false };
		let angle = self.max_slope_angle.min(std::f32::consts::FRAC_PI_2);
		normal.y > 0.0 && normal.y >= angle.cos()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn support_uses_the_same_configured_slope_as_character_movement() {
		let locomotion = CharacterLocomotion { max_slope_angle: 70.0_f32.to_radians() };
		let captured_normal = Vec3::new(0.84, 0.54, 0.0);
		assert!(locomotion.accepts_support_normal(captured_normal));
		let above_limit = Vec3::new(72.0_f32.to_radians().sin(), 72.0_f32.to_radians().cos(), 0.0);
		assert!(!locomotion.accepts_support_normal(above_limit));
		assert!(locomotion.accepts_support_normal(Vec3::Y));
	}

	#[test]
	fn support_rejects_invalid_side_and_downward_normals() {
		let locomotion = CharacterLocomotion { max_slope_angle: 70.0_f32.to_radians() };
		for normal in [Vec3::ZERO, Vec3::X, Vec3::NEG_Y, Vec3::NAN, Vec3::INFINITY] {
			assert!(!locomotion.accepts_support_normal(normal), "invalid support: {normal:?}");
		}
		for angle in [f32::NAN, f32::INFINITY, -1.0] {
			assert!(!CharacterLocomotion { max_slope_angle: angle }.accepts_support_normal(Vec3::Y));
		}
		assert!(!CharacterLocomotion { max_slope_angle: std::f32::consts::PI }
			.accepts_support_normal(Vec3::NEG_Y));
	}
}
