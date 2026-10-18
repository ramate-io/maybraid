//! Trot launch progress stored on the body rig host (animation side).

use bevy::prelude::*;

/// Progress from quadruped still into a phased run sample. `1.0` means settled.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct QuadrupedRunStartBlend {
	pub progress: f32,
}

impl QuadrupedRunStartBlend {
	pub fn settled(self) -> bool {
		self.progress >= 1.0
	}
}
