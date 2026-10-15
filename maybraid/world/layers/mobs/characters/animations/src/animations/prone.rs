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
	/// Hold-ready elbow flex at full depth (radians).
	pub arm_flex: f32,
	/// Fight-forward component of the prone arm aim at full depth (character space).
	pub arm_aim_forward: f32,
	/// Downward component of the prone arm aim at full depth (character space).
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

	/// Character-space humerus aim while settling prone (same vector on both arms).
	pub fn arm_aim_along(&self, _progress: f32) -> Vec3 {
		Vec3::new(0.0, -self.arm_aim_drop, self.arm_aim_forward).normalize()
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
	fn arm_aim_points_forward_and_down() -> anyhow::Result<()> {
		let along = Prone::default().arm_aim_along(1.0);
		assert!(along.z > 0.9, "reach stays fight-forward, got {along:?}");
		assert!(along.y < -0.2, "reach drops toward the ground plane, got {along:?}");
		assert!(along.x.abs() < 1e-6);
		Ok(())
	}
}
