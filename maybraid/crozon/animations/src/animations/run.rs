use std::f32::consts::PI;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct Run<Rig> {
	pub arm_down: f32,
	pub elbow_bend: f32,
	pub elbow_pump: f32,
	pub elbow_cycle: f32,
	pub shoulder_swing: f32,
	pub shoulder_lift: f32,
	pub hip_swing: f32,
	pub hip_lift: f32,
	pub stride: f32,
	pub humerus_swing_scale: f32,
	pub knee_neutral: f32,
	pub knee_contracted: f32,
	pub knee_extended: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Default for Run<Rig> {
	fn default() -> Self {
		Self {
			arm_down: 0.85,
			elbow_bend: 1.25,
			elbow_pump: 0.5,
			elbow_cycle: 0.35,
			shoulder_swing: 0.14,
			shoulder_lift: 0.07,
			hip_swing: 0.1,
			hip_lift: 0.06,
			stride: 1.05,
			humerus_swing_scale: 0.75,
			knee_neutral: PI * 0.5,
			knee_contracted: 2.15,
			knee_extended: 0.35,
			_rig: PhantomData,
		}
	}
}
