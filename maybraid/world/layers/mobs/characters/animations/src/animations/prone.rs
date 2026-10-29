use std::f32::consts::{FRAC_PI_2, FRAC_PI_3};

use bevy::prelude::Vec3;

/// Held prone pose. `progress` is settle depth (0 = stand, 1 = flat).
#[derive(Debug, Clone, Copy)]
pub struct Prone {
	/// Total sagittal spine pitch toward horizontal at full depth (radians).
	pub spine_peak: f32,
	/// Femur aft swing at full depth (radians).
	pub femur_peak: f32,
	/// Residual shin flex at full depth (radians).
	pub shin_peak: f32,
	/// Neck compensation so the head looks along the ground (radians).
	pub neck_peak: f32,
	/// Hold-ready arm flex at full depth (radians).
	pub arm_flex: f32,
	/// Character-space forward reach at full depth.
	pub arm_aim_forward: f32,
	/// Character-space downward reach at full depth.
	pub arm_aim_drop: f32,
}

impl Prone {
	pub fn depth(progress: f32) -> f32 {
		progress.clamp(0.0, 1.0)
	}

	pub fn spine_pitch(&self, progress: f32) -> f32 {
		Self::depth(progress) * self.spine_peak
	}

	pub fn root_swing(&self, progress: f32) -> f32 {
		self.spine_pitch(progress)
	}

	pub fn femur_swing(&self, progress: f32) -> f32 {
		Self::depth(progress) * self.femur_peak
	}

	pub fn shin_flex(&self, progress: f32) -> f32 {
		Self::depth(progress) * self.shin_peak
	}

	pub fn neck_swing(&self, progress: f32) -> f32 {
		Self::depth(progress) * self.neck_peak
	}

	pub fn arm_hold(&self, progress: f32) -> f32 {
		Self::depth(progress) * self.arm_flex
	}

	/// Move the humerus from its effective-rest direction into a low forward reach.
	pub fn arm_aim_along(&self, progress: f32, rest_direction: Vec3) -> Vec3 {
		let t = Self::depth(progress);
		let target = Vec3::new(0.0, -self.arm_aim_drop, self.arm_aim_forward).normalize();
		let rest = rest_direction.normalize_or_zero();
		let blended = rest.lerp(target, t);
		if blended.length_squared() > f32::EPSILON {
			blended.normalize()
		} else {
			target
		}
	}
}

impl Default for Prone {
	fn default() -> Self {
		Self {
			spine_peak: FRAC_PI_2,
			femur_peak: FRAC_PI_3,
			shin_peak: 0.15,
			neck_peak: -FRAC_PI_2 * 0.35,
			arm_flex: 0.35,
			arm_aim_forward: 1.0,
			arm_aim_drop: 0.4,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn depth_is_progress() -> anyhow::Result<()> {
		assert!((Prone::depth(0.0)).abs() < 1e-6);
		assert!((Prone::depth(1.0) - 1.0).abs() < 1e-6);
		assert!((Prone::depth(2.0) - 1.0).abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn arm_aim_eases_from_rest_into_forward_down_reach() -> anyhow::Result<()> {
		let prone = Prone::default();
		let left_rest = Vec3::X;
		let start = prone.arm_aim_along(0.0, left_rest);
		let mid = prone.arm_aim_along(0.5, left_rest);
		let end = prone.arm_aim_along(1.0, left_rest);
		assert!(start.dot(left_rest) > 1.0 - 1e-5);
		assert!(mid.z > 0.0 && mid.y < 0.0, "reach begins smoothly: {mid:?}");
		assert!(end.z > 0.9 && end.y < -0.2, "reach points forward and down: {end:?}");
		Ok(())
	}
}
