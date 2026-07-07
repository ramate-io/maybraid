use std::marker::PhantomData;

use crate::animations::TuckProfile;

/// Held gymnastics tuck; pose is constant across progress.
#[derive(Debug, Clone)]
pub struct FixedTuck<Rig> {
	/// `1.0` matches the tuned default tuck shape.
	pub tightness: f32,
	_rig: PhantomData<Rig>,
}

impl<Rig> Default for FixedTuck<Rig> {
	fn default() -> Self {
		Self { tightness: TuckProfile::DEFAULT_TIGHTNESS, _rig: PhantomData }
	}
}

impl<Rig> FixedTuck<Rig> {
	pub fn new(tightness: f32) -> Self {
		Self { tightness, _rig: PhantomData }
	}

	pub fn tightness(&self) -> f32 {
		self.tightness
	}

	pub fn profile(&self) -> TuckProfile {
		TuckProfile::new(self.tightness)
	}
}
