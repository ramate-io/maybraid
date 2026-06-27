use std::marker::PhantomData;

use crate::animations::Squat;

const SHOULDER_SWING_BACK: f32 = -0.55;
const HUMERUS_FLEX_BACK: f32 = 0.65;
const FOREARM_EXTEND: f32 = -0.35;

#[derive(Debug, Clone)]
pub struct Spring<Rig> {
	pub phase: f32,
	pub squat: Squat<Rig>,
	_rig: PhantomData<Rig>,
}

impl<Rig> Spring<Rig> {
	pub fn new(phase: f32, squat: Squat<Rig>) -> Self {
		Self { phase, squat, _rig: PhantomData }
	}

	/// Ease-out 0 at crouch, 1 at full extension.
	pub fn extend_amount(&self) -> f32 {
		let t = self.phase.fract();
		1.0 - (1.0 - t).powi(2)
	}

	pub fn femur_swing(&self) -> f32 {
		self.squat.femur_peak * (1.0 - self.extend_amount())
	}

	pub fn shin_flex(&self) -> f32 {
		self.squat.shin_peak * (1.0 - self.extend_amount())
	}

	pub fn root_swing(&self) -> f32 {
		self.squat.root_peak * (1.0 - self.extend_amount())
	}

	pub fn arm_amount(&self) -> f32 {
		self.extend_amount()
	}

	pub fn shoulder_swing(&self) -> f32 {
		self.arm_amount() * SHOULDER_SWING_BACK
	}

	pub fn humerus_flex(&self) -> f32 {
		self.arm_amount() * HUMERUS_FLEX_BACK
	}

	pub fn forearm_flex(&self) -> f32 {
		self.arm_amount() * FOREARM_EXTEND
	}

	/// Squat-depth vertical drop at spring start (legs fully bent).
	pub fn start_drop(&self, lengths: crozon_rigs::humanoid::LegSegmentLengths) -> f32 {
		self.squat.vertical_drop(lengths)
	}
}

impl<Rig> Default for Spring<Rig> {
	fn default() -> Self {
		Self::new(0.0, Squat::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn spring_end_straens_legs() -> anyhow::Result<()> {
		let spring = Spring::<()>::new(0.99, Squat::default());
		assert!(spring.femur_swing().abs() < 1e-2);
		assert!(spring.shin_flex().abs() < 1e-2);
		assert!(spring.root_swing().abs() < 1e-2);
		Ok(())
	}

	#[test]
	fn spring_start_matches_full_squat_angles() -> anyhow::Result<()> {
		let squat = Squat::<()>::default();
		let spring = Spring::<()>::new(0.0, squat.clone());
		assert!((spring.femur_swing() - squat.femur_peak).abs() < 1e-5);
		assert!((spring.shin_flex() - squat.shin_peak).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn spring_arms_reach_back_at_full_extension() -> anyhow::Result<()> {
		let spring = Spring::<()>::new(0.99, Squat::default());
		assert!(spring.shoulder_swing() < -0.3);
		assert!(spring.humerus_flex() > 0.0);
		Ok(())
	}
}
