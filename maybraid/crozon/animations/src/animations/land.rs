use std::f32::consts::PI;
use std::marker::PhantomData;

use crate::animations::{Squat, vertical_drop};

const DEFAULT_LAND_SCALE: f32 = 0.35;

#[derive(Debug, Clone)]
pub struct Land<Rig> {
	pub phase: f32,
	pub scale: f32,
	pub squat: Squat<Rig>,
	_rig: PhantomData<Rig>,
}

impl<Rig> Land<Rig> {
	pub fn new(phase: f32, squat: Squat<Rig>) -> Self {
		Self { phase, scale: DEFAULT_LAND_SCALE, squat, _rig: PhantomData }
	}

	pub fn land_amount(&self) -> f32 {
		(self.phase.fract() * PI).sin()
	}

	pub fn femur_swing(&self) -> f32 {
		self.land_amount() * self.scale * self.squat.femur_peak
	}

	pub fn shin_flex(&self) -> f32 {
		self.land_amount() * self.scale * self.squat.shin_peak
	}

	pub fn root_swing(&self) -> f32 {
		self.land_amount() * self.scale * self.squat.root_peak
	}

	pub fn vertical_drop(&self, lengths: crozon_rigs::humanoid::LegSegmentLengths) -> f32 {
		vertical_drop(self.femur_swing(), self.shin_flex(), lengths)
	}
}

impl<Rig> Default for Land<Rig> {
	fn default() -> Self {
		Self::new(0.0, Squat::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::Squat;

	#[test]
	fn land_peak_flex_below_full_squat() -> anyhow::Result<()> {
		let squat = Squat::<()>::new(0.5);
		let land = Land::<()>::new(0.5, Squat::default());
		assert!(land.femur_swing().abs() < squat.femur_swing().abs());
		assert!(land.shin_flex().abs() < squat.shin_flex().abs());
		Ok(())
	}

	#[test]
	fn land_endpoints_are_neutral() -> anyhow::Result<()> {
		let land = Land::<()>::new(0.0, Squat::default());
		assert!(land.femur_swing().abs() < 1e-5);
		let land = Land::<()>::new(1.0, Squat::default());
		assert!(land.femur_swing().abs() < 1e-5);
		Ok(())
	}
}
