use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct Run<Rig> {
	pub phase: f32,
	pub stride: f32,
	pub arm_swing: f32,
	pub arm_down: f32,
	pub elbow_bend: f32,
	pub elbow_pump: f32,
	pub hip_swing: f32,
	pub knee_neutral: f32,
	pub knee_contracted: f32,
	pub knee_extended: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Run<Rig> {
	pub fn new(phase: f32) -> Self {
		Self { phase, ..Self::default() }
	}
}

impl<Rig> Default for Run<Rig> {
	fn default() -> Self {
		Self {
			phase: 0.0,
			stride: 1.05,
			arm_swing: 0.75,
			arm_down: 0.85,
			elbow_bend: 1.25,
			elbow_pump: 0.5,
			hip_swing: 0.1,
			knee_neutral: std::f32::consts::PI * 0.5,
			knee_contracted: 2.15,
			knee_extended: 0.35,
			_rig: PhantomData,
		}
	}
}
