use std::marker::PhantomData;

use crate::Progress;

const SHOULDER_FLEX_SPREAD: f32 = 0.75;
const HUMERUS_SWING_SPREAD: f32 = 0.45;
const FOREARM_EXTEND: f32 = -0.2;

#[derive(Debug, Clone)]
pub struct Fall<Rig> {
	_rig: PhantomData<Rig>,
}

impl<Rig> Default for Fall<Rig> {
	fn default() -> Self {
		Self { _rig: PhantomData }
	}
}

impl<Rig> Fall<Rig> {
	/// Ramp in during the first fifth of progress, then hold through `1.0`.
	pub fn spread_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < 0.2 {
			t / 0.2
		} else {
			1.0
		}
	}

	pub fn shoulder_flex(&self, side: crozon_rigs::Side, progress: f32) -> f32 {
		let sign = match side {
			crozon_rigs::Side::Left => 1.0,
			crozon_rigs::Side::Right => -1.0,
		};
		self.spread_amount(progress) * SHOULDER_FLEX_SPREAD * sign
	}

	pub fn humerus_swing(&self, side: crozon_rigs::Side, progress: f32) -> f32 {
		let sign = match side {
			crozon_rigs::Side::Left => -1.0,
			crozon_rigs::Side::Right => 1.0,
		};
		self.spread_amount(progress) * HUMERUS_SWING_SPREAD * sign
	}

	pub fn forearm_flex(&self, progress: f32) -> f32 {
		self.spread_amount(progress) * FOREARM_EXTEND
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::Side;

	use super::*;

	#[test]
	fn fall_spreads_arms_symmetrically() -> anyhow::Result<()> {
		let fall = Fall::<()>::default();
		assert!(fall.shoulder_flex(Side::Left, 0.5).abs() > 0.1);
		assert!(fall.shoulder_flex(Side::Right, 0.5).abs() > 0.1);
		assert!(
			fall.shoulder_flex(Side::Left, 0.5).signum()
				!= fall.shoulder_flex(Side::Right, 0.5).signum()
		);
		Ok(())
	}

	#[test]
	fn fall_legs_stay_extended() -> anyhow::Result<()> {
		let fall = Fall::<()>::default();
		assert_eq!(fall.spread_amount(0.5), 1.0);
		Ok(())
	}
}
