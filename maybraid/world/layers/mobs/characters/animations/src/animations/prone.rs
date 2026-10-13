use std::f32::consts::{FRAC_PI_2, FRAC_PI_3};

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
	/// Shoulder-forward reach along the ground at full depth (radians).
	pub arm_reach_peak: f32,
	/// Extra elbow flex on the left; subtracted on the right for a natural offset.
	pub arm_elbow_bias: f32,
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

	pub fn arm_reach(&self, progress: f32) -> f32 {
		Self::depth(progress) * self.arm_reach_peak
	}

	pub fn arm_humerus_forward(&self, progress: f32) -> f32 {
		self.arm_reach(progress) * 0.4
	}

	pub fn arm_humerus_lateral(&self, progress: f32) -> f32 {
		self.arm_hold(progress) * 0.7
	}

	pub fn arm_elbow_flex(&self, side: character_rigs::Side, progress: f32) -> f32 {
		let base = self.arm_hold(progress);
		let bias = match side {
			character_rigs::Side::Left => self.arm_elbow_bias,
			character_rigs::Side::Right => -self.arm_elbow_bias,
		};
		base + bias
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
			arm_reach_peak: 0.30,
			arm_elbow_bias: 0.04,
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
}
