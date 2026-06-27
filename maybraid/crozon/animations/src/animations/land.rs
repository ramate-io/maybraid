use std::marker::PhantomData;

use crozon_rigs::humanoid::LegSegmentLengths;

use crate::animations::{vertical_drop, Squat};

const DEFAULT_LAND_SCALE: f32 = 0.35;
/// Baseline landing recovery rate before jump-height scaling.
pub const DEFAULT_RECOVERY_SPEED: f32 = 0.6;

#[derive(Debug, Clone)]
pub struct Land<Rig> {
	pub squat: Squat<Rig>,
	pub scale: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Land<Rig> {
	pub fn with_speeds(descent_speed: f32, ascent_speed: f32, squat: Squat<Rig>) -> Self {
		let mut timed = Squat::with_speeds(descent_speed, ascent_speed);
		timed.femur_peak = squat.femur_peak;
		timed.shin_peak = squat.shin_peak;
		timed.root_peak = squat.root_peak;
		Self { squat: timed, scale: DEFAULT_LAND_SCALE, _rig: PhantomData }
	}

	pub fn at_segment_time(mut self, segment_time: f32) -> Self {
		self.squat = self.squat.at_segment_time(segment_time);
		self
	}

	pub fn depth(&self) -> f32 {
		self.squat.depth()
	}

	pub fn descent_duration(&self) -> f32 {
		self.squat.descent_duration()
	}

	pub fn ascent_duration(&self) -> f32 {
		self.squat.ascent_duration()
	}

	pub fn femur_swing(&self) -> f32 {
		self.depth() * self.scale * self.squat.femur_peak
	}

	pub fn shin_flex(&self) -> f32 {
		self.depth() * self.scale * self.squat.shin_peak
	}

	pub fn root_swing(&self) -> f32 {
		self.depth() * self.scale * self.squat.root_peak
	}

	pub fn vertical_drop(&self, lengths: LegSegmentLengths) -> f32 {
		vertical_drop(self.femur_swing(), self.shin_flex(), lengths)
	}

	pub fn peak_vertical_drop(&self, lengths: LegSegmentLengths) -> f32 {
		vertical_drop(
			self.scale * self.squat.femur_peak,
			self.scale * self.squat.shin_peak,
			lengths,
		)
	}
}

impl<Rig> Default for Land<Rig> {
	fn default() -> Self {
		Self::with_speeds(DEFAULT_RECOVERY_SPEED, DEFAULT_RECOVERY_SPEED, Squat::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::Squat;

	#[test]
	fn land_peak_flex_below_full_squat() -> anyhow::Result<()> {
		let squat = Squat::<()>::new(0.5);
		let land = Land::<()>::with_speeds(1.0, 1.0, Squat::default()).at_segment_time(1.0);
		assert!(land.femur_swing().abs() < squat.femur_swing().abs());
		assert!(land.shin_flex().abs() < squat.shin_flex().abs());
		Ok(())
	}

	#[test]
	fn land_starts_at_stand() -> anyhow::Result<()> {
		let land = Land::<()>::with_speeds(1.0, 1.0, Squat::default()).at_segment_time(0.0);
		assert!(land.depth().abs() < 1e-5);
		assert!(land.femur_swing().abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn land_compresses_gradually_after_touchdown() -> anyhow::Result<()> {
		let land = Land::<()>::with_speeds(1.0, 1.0, Squat::default()).at_segment_time(0.05);
		assert!(land.depth() > 0.0);
		assert!(land.depth() < 0.2);
		Ok(())
	}
}
