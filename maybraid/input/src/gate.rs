//! Gameplay suppress flag. Text entry / modal hosts set this false.

use bevy::prelude::*;

/// When false, analog and pad buttons are cleared after produce. The keyboard
/// overlay stays so debug keys and text hosts can still see physical keys.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PadGameplayEnabled(pub bool);

impl Default for PadGameplayEnabled {
	fn default() -> Self {
		Self(true)
	}
}

impl PadGameplayEnabled {
	pub fn is_enabled(self) -> bool {
		self.0
	}
}
