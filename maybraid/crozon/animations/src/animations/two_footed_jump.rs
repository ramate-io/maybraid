use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct TwoFootedJump<Rig> {
	pub phase: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> TwoFootedJump<Rig> {
	pub fn new(phase: f32) -> Self {
		Self { phase, _rig: PhantomData }
	}
}
