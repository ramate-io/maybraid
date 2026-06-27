use std::marker::PhantomData;

use crozon_rigs::humanoid::LegSegmentLengths;

use crate::animations::{vertical_drop, Squat};

/// Peak landing flex as a fraction of a full pre-jump squat.
const DEFAULT_LAND_SCALE: f32 = 0.35;

#[derive(Debug, Clone)]
pub struct Land<Rig> {
	/// Timed squat envelope for landing compression and recovery.
	pub squat: Squat<Rig>,
	/// Scales peak landing joint flex relative to a full squat.
	pub scale: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Land<Rig> {
	/// Landing squat with explicit compression and recovery half-cycle speeds.
	pub fn with_speeds(descent_speed: f32, ascent_speed: f32, squat: Squat<Rig>) -> Self {
		let mut timed = Squat::with_speeds(descent_speed, ascent_speed);
		timed.femur_peak = squat.femur_peak;
		timed.shin_peak = squat.shin_peak;
		timed.root_peak = squat.root_peak;
		Self { squat: timed, scale: DEFAULT_LAND_SCALE, _rig: PhantomData }
	}

	/// Sample the landing envelope at `segment_time` seconds after touchdown.
	pub fn at_segment_time(mut self, segment_time: f32) -> Self {
		self.squat = self.squat.at_segment_time(segment_time);
		self
	}

	/// Normalized landing depth before scale: 0 at touch-down extension, 1 at peak compression.
	pub fn depth(&self) -> f32 {
		self.squat.depth()
	}

	/// Duration of impact compression (touch-down to peak flex) in seconds.
	pub fn descent_duration(&self) -> f32 {
		self.squat.descent_duration()
	}

	/// Duration of stand-up recovery after peak compression in seconds.
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

	/// Vertical drop at peak landing compression.
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
		Self::with_speeds(1.0, 1.0, Squat::default())
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
		let land = Land::<()>::with_speeds(10.0, 1.0, Squat::default()).at_segment_time(0.05);
		assert!(land.depth() > 0.0);
		Ok(())
	}
}
