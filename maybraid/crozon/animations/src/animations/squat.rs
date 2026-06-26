use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::marker::PhantomData;

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

	pub fn femur_swing(&self) -> f32 {
		self.squat_amount() * self.femur_peak
	}

	pub fn shin_flex(&self) -> f32 {
		self.squat_amount() * self.shin_peak
	}

	pub fn root_swing(&self) -> f32 {
		self.squat_amount() * self.root_peak
	}
}
