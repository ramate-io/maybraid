use crate::Progress;

const SHOULDER_FLEX_SPREAD: f32 = 0.75;
const HUMERUS_SWING_SPREAD: f32 = 0.45;
const FOREARM_EXTEND: f32 = -0.2;
/// Spread finishes partway through the clip so jump air time does not pop to full T early.
const SPREAD_RAMP_END: f32 = 0.35;

#[derive(Debug, Clone, Default)]
pub struct Fall;

impl Fall {
	/// Ramp in during the first fifth of progress, then hold through `1.0`.
	pub fn spread_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
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
		self.spread_amount(progress) * SHOULDER_FLEX_SPREAD * sign
	}

	pub fn humerus_swing(&self, side: character_rigs::Side, progress: f32) -> f32 {
		let sign = match side {
			character_rigs::Side::Left => -1.0,
			character_rigs::Side::Right => 1.0,
		};
		self.spread_amount(progress) * HUMERUS_SWING_SPREAD * sign
	}

	pub fn forearm_flex(&self, progress: f32) -> f32 {
		self.spread_amount(progress) * FOREARM_EXTEND
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
		assert_eq!(fall.spread_amount(0.5), 1.0);
		Ok(())
	}

	#[test]
	fn fall_spread_still_ramping_after_spring_blend_window() -> anyhow::Result<()> {
		let fall = Fall::default();
		let mid_air = fall.spread_amount(0.25);
		assert!(mid_air > 0.65 && mid_air < 0.95, "expected partial spread at 0.25, got {mid_air}");
		assert_eq!(fall.spread_amount(1.0), 1.0);
		Ok(())
	}

	#[test]
	fn fall_spread_mirrors_left_and_right_shoulder_flex() -> anyhow::Result<()> {
		let fall = Fall::default();
		let progress = 0.3;
		let left = fall.shoulder_flex(Side::Left, progress);
		let right = fall.shoulder_flex(Side::Right, progress);
		assert!(left.abs() > 0.05 && right.abs() > 0.05);
		assert_eq!(left.signum(), -right.signum());
		Ok(())
	}
}
