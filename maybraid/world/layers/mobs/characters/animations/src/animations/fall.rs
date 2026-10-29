use crate::Progress;

const SHOULDER_FLEX_SPREAD: f32 = 0.75;
const HUMERUS_SWING_SPREAD: f32 = 0.45;
const FOREARM_EXTEND: f32 = -0.2;
const RIGHT_ARM_SPREAD_DELAY: f32 = 0.05;
const SPREAD_RAMP_END: f32 = 0.35;

#[derive(Debug, Clone, Default)]
pub struct Fall;

impl Fall {
	fn spread_delay(&self, side: character_rigs::Side) -> f32 {
		match side {
			character_rigs::Side::Left => 0.0,
			character_rigs::Side::Right => RIGHT_ARM_SPREAD_DELAY,
		}
	}

	/// Spread ramp completes at local progress 0.35; right starts 0.05 after left.
	pub fn spread_amount(&self, side: character_rigs::Side, progress: f32) -> f32 {
		let t = Progress(progress - self.spread_delay(side)).clamp();
		if t < SPREAD_RAMP_END {
			t / SPREAD_RAMP_END
		} else {
			1.0
		}
	}

	pub fn shoulder_flex(&self, side: character_rigs::Side, progress: f32) -> f32 {
		let sign = match side {
			character_rigs::Side::Left => 1.0,
			character_rigs::Side::Right => -1.0,
		};
		self.spread_amount(side, progress) * SHOULDER_FLEX_SPREAD * sign
	}

	pub fn humerus_swing(&self, side: character_rigs::Side, progress: f32) -> f32 {
		let sign = match side {
			character_rigs::Side::Left => -1.0,
			character_rigs::Side::Right => 1.0,
		};
		self.spread_amount(side, progress) * HUMERUS_SWING_SPREAD * sign
	}

	pub fn forearm_flex(&self, side: character_rigs::Side, progress: f32) -> f32 {
		self.spread_amount(side, progress) * FOREARM_EXTEND
	}
}

#[cfg(test)]
mod tests {
	use character_rigs::Side;

	use super::*;

	#[test]
	fn fall_spreads_arms_symmetrically() -> anyhow::Result<()> {
		let fall = Fall::default();
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
		let fall = Fall::default();
		assert_eq!(fall.spread_amount(Side::Left, 0.5), 1.0);
		assert!(fall.spread_amount(Side::Left, 0.25) < 1.0);
		assert!(fall.spread_amount(Side::Left, 0.35) == 1.0);
		Ok(())
	}

	#[test]
	fn right_spread_trails_left_and_catches_up_after_the_ramp() -> anyhow::Result<()> {
		let fall = Fall::default();
		let left = fall.spread_amount(Side::Left, 0.20);
		let right = fall.spread_amount(Side::Right, 0.20);
		assert!(left > right + 0.1, "left leads the stagger, L={left} R={right}");
		assert_eq!(fall.spread_amount(Side::Left, 0.40), 1.0);
		assert_eq!(fall.spread_amount(Side::Right, 0.40), 1.0);
		Ok(())
	}
}
