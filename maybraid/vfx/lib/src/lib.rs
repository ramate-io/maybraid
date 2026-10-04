//! Real-time VFX: particles, flipbooks, ribbons, and short-lived blast hosts.
//!
//! Domain shaders and world materials stay in their crates. This lib is the
//! effect graph — concepts land here before they are wired into firearms or world.

use bevy::prelude::*;
use bevy_hanabi::HanabiPlugin;

/// Registers the GPU particle stack. Idempotent.
pub struct VfxPlugin;

impl Plugin for VfxPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<HanabiPlugin>() {
			app.add_plugins(HanabiPlugin);
		}
	}
}
