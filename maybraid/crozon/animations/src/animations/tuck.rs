use std::marker::PhantomData;

use crozon_rigs::Side;

use crate::Progress;

/// Default femur forward swing at full tuck (radians).
const FEMUR_TUCK: f32 = -0.55;
/// Default shin flex at full tuck (radians).
const SHIN_TUCK: f32 = 2.2;
/// Shoulder flex pulling the arm toward the torso (mirrors [`Fall`](super::Fall) spread axis).
const SHOULDER_FLEX_TUCK: f32 = 0.65;
/// Humerus swing closing the upper arm (mirrors fall spread on the swing axis).
const HUMERUS_SWING_TUCK: f32 = 0.55;
/// Forearm flex at full tuck.
const FOREARM_FLEX_TUCK: f32 = 1.4;

#[derive(Debug, Clone)]
pub struct Tuck<Rig> {
	/// Peak femur forward swing at full tuck (radians).
	pub femur_tuck: f32,
	/// Peak shin flex at full tuck (radians).
	pub shin_tuck: f32,
	/// Shoulder flex magnitude at full tuck (radians).
	pub shoulder_flex_tuck: f32,
	/// Humerus swing magnitude at full tuck (radians).
	pub humerus_swing_tuck: f32,
	/// Forearm flex magnitude at full tuck (radians).
	pub forearm_flex_tuck: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Default for Tuck<Rig> {
	fn default() -> Self {
		Self {
			femur_tuck: FEMUR_TUCK,
			shin_tuck: SHIN_TUCK,
			shoulder_flex_tuck: SHOULDER_FLEX_TUCK,
			humerus_swing_tuck: HUMERUS_SWING_TUCK,
			forearm_flex_tuck: FOREARM_FLEX_TUCK,
			_rig: PhantomData,
		}
	}
}

impl<Rig> Tuck<Rig> {
	/// Ramp in during the first 15% of progress, then hold through `1.0`.
	pub fn tuck_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < 0.15 {
			t / 0.15
		} else {
			1.0
		}
	}

	pub fn femur_swing(&self, progress: f32) -> f32 {
		self.tuck_amount(progress) * self.femur_tuck
	}

	pub fn shin_flex(&self, progress: f32) -> f32 {
		self.tuck_amount(progress) * self.shin_tuck
	}

	/// Opposite side signs from fall spread so the arms close over the chest.
	pub fn shoulder_flex(&self, side: Side, progress: f32) -> f32 {
		let sign = match side {
			Side::Left => -1.0,
			Side::Right => 1.0,
		};
		self.tuck_amount(progress) * self.shoulder_flex_tuck * sign
	}

	pub fn humerus_swing(&self, side: Side, progress: f32) -> f32 {
		let sign = match side {
			Side::Left => 1.0,
			Side::Right => -1.0,
		};
		self.tuck_amount(progress) * self.humerus_swing_tuck * sign
	}

	pub fn forearm_flex(&self, progress: f32) -> f32 {
		self.tuck_amount(progress) * self.forearm_flex_tuck
	}
}

#[cfg(test)]
mod tests {
	use std::f32::consts::FRAC_PI_2;

	use crozon_rigs::Side;

	use super::*;
	use crate::animations::Fall;

	#[test]
	fn tuck_amount_ramps_then_holds() -> anyhow::Result<()> {
		let tuck = Tuck::<()>::default();
		assert!(tuck.tuck_amount(0.0).abs() < 1e-5);
		assert!((tuck.tuck_amount(0.15) - 1.0).abs() < 1e-5);
		assert_eq!(tuck.tuck_amount(0.5), 1.0);
		Ok(())
	}

	#[test]
	fn tuck_flexes_knees_beyond_squat_depth() -> anyhow::Result<()> {
		let tuck = Tuck::<()>::default();
		assert!(tuck.shin_flex(0.5).abs() > FRAC_PI_2);
		assert!(tuck.femur_swing(0.5).abs() > 0.2);
		Ok(())
	}

	#[test]
	fn tuck_arms_mirror_fall_axes_with_opposite_signs() -> anyhow::Result<()> {
		let tuck = Tuck::<()>::default();
		let fall = Fall::<()>::default();
		for side in [Side::Left, Side::Right] {
			assert!(
				tuck.shoulder_flex(side, 0.5).signum()
					!= fall.shoulder_flex(side, 0.5).signum()
			);
			assert!(
				tuck.humerus_swing(side, 0.5).signum() != fall.humerus_swing(side, 0.5).signum()
			);
		}
		Ok(())
	}
}
