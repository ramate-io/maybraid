use std::marker::PhantomData;

const SHOULDER_FLEX_SPREAD: f32 = 0.75;
const HUMERUS_SWING_SPREAD: f32 = 0.45;
const FOREARM_EXTEND: f32 = -0.2;

#[derive(Debug, Clone)]
pub struct Fall<Rig> {
	pub phase: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Fall<Rig> {
	pub fn new(phase: f32) -> Self {
		Self { phase, _rig: PhantomData }
	}

	/// Full airborne spread pose (for mix targets).
	pub fn spread() -> Self {
		Self::new(1.0)
	}

	/// Ramp in during the first fifth of the segment, then hold.
	pub fn spread_amount(&self) -> f32 {
		let t = self.phase.clamp(0.0, 1.0);
		if t < 0.2 {
			t / 0.2
		} else {
			1.0
		}
	}

	pub fn shoulder_flex(&self, side: crozon_rigs::Side) -> f32 {
		let sign = match side {
			crozon_rigs::Side::Left => 1.0,
			crozon_rigs::Side::Right => -1.0,
		};
		self.spread_amount() * SHOULDER_FLEX_SPREAD * sign
	}

	pub fn humerus_swing(&self, side: crozon_rigs::Side) -> f32 {
		let sign = match side {
			crozon_rigs::Side::Left => -1.0,
			crozon_rigs::Side::Right => 1.0,
		};
		self.spread_amount() * HUMERUS_SWING_SPREAD * sign
	}

	pub fn forearm_flex(&self) -> f32 {
		self.spread_amount() * FOREARM_EXTEND
	}
}

impl<Rig> Default for Fall<Rig> {
	fn default() -> Self {
		Self::new(0.0)
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::Side;

	use super::*;

	#[test]
	fn fall_spreads_arms_symmetrically() -> anyhow::Result<()> {
		let fall = Fall::<()>::new(0.5);
		assert!(fall.shoulder_flex(Side::Left).abs() > 0.1);
		assert!(fall.shoulder_flex(Side::Right).abs() > 0.1);
		assert!(
			fall.shoulder_flex(Side::Left).signum() != fall.shoulder_flex(Side::Right).signum()
		);
		Ok(())
	}

	#[test]
	fn fall_legs_stay_extended() -> anyhow::Result<()> {
		let fall = Fall::<()>::new(0.5);
		assert_eq!(fall.spread_amount(), 1.0);
		Ok(())
	}
}
