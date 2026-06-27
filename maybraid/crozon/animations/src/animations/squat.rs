use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::marker::PhantomData;

use crozon_rigs::humanoid::LegSegmentLengths;

const ROOT_SQUAT_DEG: f32 = 15.0;

#[derive(Debug, Clone)]
pub struct Squat<Rig> {
	pub phase: f32,
	/// Peak femur forward angle at deepest squat (radians).
	pub femur_peak: f32,
	/// Peak shin angle relative to femur at deepest squat (radians).
	pub shin_peak: f32,
	/// Peak root forward angle at deepest squat (radians).
	pub root_peak: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Squat<Rig> {
	pub fn new(phase: f32) -> Self {
		Self { phase, ..Self::default() }
	}

	pub fn from_time(t: f32, cycle_speed: f32) -> Self {
		Self::new((t * cycle_speed).fract())
	}
}

impl<Rig> Default for Squat<Rig> {
	fn default() -> Self {
		Self {
			phase: 0.0,
			femur_peak: -FRAC_PI_4,
			shin_peak: FRAC_PI_2,
			root_peak: ROOT_SQUAT_DEG.to_radians(),
			_rig: PhantomData,
		}
	}
}

impl<Rig> Squat<Rig> {
	/// 0 at stand, 1 at deepest squat, back to 0 over one cycle.
	pub fn squat_amount(&self) -> f32 {
		(self.phase.fract() * PI).sin()
	}

	/// Wind-up only: 0 at stand, 1 at deepest squat (no return).
	pub fn wind_up_amount(&self) -> f32 {
		(self.phase.fract() * FRAC_PI_2).sin()
	}

	pub fn wind_up_femur_swing(&self) -> f32 {
		self.wind_up_amount() * self.femur_peak
	}

	pub fn wind_up_shin_flex(&self) -> f32 {
		self.wind_up_amount() * self.shin_peak
	}

	pub fn wind_up_root_swing(&self) -> f32 {
		self.wind_up_amount() * self.root_peak
	}

	pub fn wind_up_vertical_drop(&self, lengths: LegSegmentLengths) -> f32 {
		vertical_drop(self.wind_up_femur_swing(), self.wind_up_shin_flex(), lengths)
	}

	pub fn femur_swing(&self) -> f32 {
		self.squat_amount() * self.femur_peak
	}

	pub fn shin_flex(&self) -> f32 {
		self.squat_amount() * self.shin_peak
	}

	pub fn root_swing(&self) -> f32 {
		self.squat_amount() * self.root_peak
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
		let drop_unit = squat.vertical_drop(unit);
		let drop_doubled = squat.vertical_drop(doubled);
		assert!((drop_doubled - drop_unit * 2.0).abs() < 1e-4);
	}
}
