use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
use std::marker::PhantomData;

use crozon_rigs::humanoid::LegSegmentLengths;

const ROOT_SQUAT_DEG: f32 = 15.0;

/// Baseline windup descent rate before jump-height scaling.
pub const DEFAULT_WINDUP_DESCENT_SPEED: f32 = 0.9;

#[derive(Debug, Clone)]
pub struct Squat<Rig> {
	pub time: f32,
	/// Stand to full depth in `1/descent_speed` seconds.
	pub descent_speed: f32,
	/// Full depth to stand in `1/ascent_speed` seconds.
	pub ascent_speed: f32,
	pub one_shot: bool,
	pub femur_peak: f32,
	pub shin_peak: f32,
	pub root_peak: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Squat<Rig> {
	/// Looping squat with independent half-cycle speeds.
	pub fn from_time(time: f32, descent_speed: f32, ascent_speed: f32) -> Self {
		Self {
			time,
			descent_speed,
			ascent_speed,
			one_shot: false,
			..Self::default()
		}
	}

	/// Alias for [`Self::from_time`].
	pub fn for_loop(time: f32, descent_speed: f32, ascent_speed: f32) -> Self {
		Self::from_time(time, descent_speed, ascent_speed)
	}

	/// Snapshot at normalized cycle phase in `[0, 1)` using unit half speeds.
	pub fn new(phase: f32) -> Self {
		let descent_speed = 1.0;
		let ascent_speed = 1.0;
		let cycle = 1.0 / descent_speed + 1.0 / ascent_speed;
		Self {
			time: phase.fract() * cycle,
			descent_speed,
			ascent_speed,
			one_shot: false,
			..Self::default()
		}
	}

	/// One-shot down-up envelope for jump segments.
	pub fn with_speeds(descent_speed: f32, ascent_speed: f32) -> Self {
		Self {
			descent_speed,
			ascent_speed,
			one_shot: true,
			..Self::default()
		}
	}

	pub fn at_segment_time(mut self, segment_time: f32) -> Self {
		self.time = segment_time;
		self.one_shot = true;
		self
	}

	pub fn cycle_phase(&self) -> f32 {
		let cycle = self.cycle_duration();
		if cycle <= f32::EPSILON {
			return 0.0;
		}
		(self.envelope_time() / cycle).fract()
	}

	pub fn cycle_duration(&self) -> f32 {
		self.descent_duration() + self.ascent_duration()
	}

	pub fn descent_duration(&self) -> f32 {
		(1.0 / self.descent_speed).max(f32::EPSILON)
	}

	pub fn ascent_duration(&self) -> f32 {
		(1.0 / self.ascent_speed).max(f32::EPSILON)
	}

	pub fn peak_vertical_drop(&self, lengths: LegSegmentLengths) -> f32 {
		vertical_drop(self.femur_peak, self.shin_peak, lengths)
	}
}

impl<Rig> Default for Squat<Rig> {
	fn default() -> Self {
		Self {
			time: 0.0,
			descent_speed: DEFAULT_WINDUP_DESCENT_SPEED,
			ascent_speed: DEFAULT_WINDUP_DESCENT_SPEED,
			one_shot: false,
			femur_peak: -FRAC_PI_4,
			shin_peak: FRAC_PI_2,
			root_peak: ROOT_SQUAT_DEG.to_radians(),
			_rig: PhantomData,
		}
	}
}

impl<Rig> Squat<Rig> {
	fn envelope_time(&self) -> f32 {
		let cycle = self.cycle_duration();
		if self.one_shot {
			self.time.clamp(0.0, cycle)
		} else {
			self.time % cycle
		}
	}

	/// 0 at stand, 1 at deepest squat.
	pub fn depth(&self) -> f32 {
		let desc_d = self.descent_duration();
		let asc_d = self.ascent_duration();
		let cycle = desc_d + asc_d;
		let t = self.envelope_time();
		if self.one_shot && self.time >= cycle {
			return 0.0;
		}
		if t <= desc_d {
			let u = (t / desc_d).clamp(0.0, 1.0);
			(u * FRAC_PI_2).sin()
		} else {
			let u = ((t - desc_d) / asc_d).clamp(0.0, 1.0);
			(u * FRAC_PI_2).cos()
		}
	}

	pub fn femur_swing(&self) -> f32 {
		self.depth() * self.femur_peak
	}

	pub fn shin_flex(&self) -> f32 {
		self.depth() * self.shin_peak
	}

	pub fn root_swing(&self) -> f32 {
		self.depth() * self.root_peak
	}

	pub fn vertical_drop(&self, lengths: LegSegmentLengths) -> f32 {
		vertical_drop(self.femur_swing(), self.shin_flex(), lengths)
	}
}

/// Two-link leg height loss from hip to ankle in the sagittal plane.
pub fn vertical_drop(femur_swing: f32, shin_flex: f32, lengths: LegSegmentLengths) -> f32 {
	let standing = lengths.femur + lengths.shin;
	let bent = lengths.femur * femur_swing.cos() + lengths.shin * (femur_swing + shin_flex).cos();
	(standing - bent).max(0.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn independent_half_speeds_stretch_descent() -> anyhow::Result<()> {
		let slow = Squat::<()>::from_time(0.5, 0.25, 1.0);
		let fast = Squat::<()>::from_time(0.5, 1.0, 1.0);
		assert!(slow.depth() < fast.depth());
		Ok(())
	}

	#[test]
	fn segment_starts_and_ends_at_stand() -> anyhow::Result<()> {
		let start = Squat::<()>::with_speeds(0.5, 0.5).at_segment_time(0.0);
		assert!(start.depth().abs() < 1e-5);
		let end = Squat::<()>::with_speeds(0.5, 0.5).at_segment_time(10.0);
		assert!(end.depth().abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn segment_peaks_at_end_of_descent() -> anyhow::Result<()> {
		let squat = Squat::<()>::with_speeds(1.0, 1.0).at_segment_time(1.0);
		assert!((squat.depth() - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn stand_phase_has_zero_drop() {
		let squat = Squat::<()>::new(0.0);
		assert_eq!(squat.vertical_drop(LegSegmentLengths::default()), 0.0);
	}

	#[test]
	fn deepest_squat_has_positive_drop() {
		let squat = Squat::<()>::new(0.5);
		assert!(squat.vertical_drop(LegSegmentLengths::default()) > 0.0);
	}

	#[test]
	fn doubling_segment_lengths_doubles_drop() {
		let squat = Squat::<()>::new(0.5);
		let unit = LegSegmentLengths { femur: 0.5, shin: 0.5 };
		let doubled = LegSegmentLengths { femur: 1.0, shin: 1.0 };
		assert!((squat.vertical_drop(doubled) - squat.vertical_drop(unit) * 2.0).abs() < 1e-4);
	}
}
