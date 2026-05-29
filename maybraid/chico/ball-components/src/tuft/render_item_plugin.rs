//! Tuft render items spawn [`Mesh3d`](bevy::prelude::Mesh3d) children directly (no SDF cache).

use bevy::prelude::*;

pub struct ChicoTuftRenderItemPlugin;

impl Default for ChicoTuftRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

impl Plugin for ChicoTuftRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MaterialPlugin<StandardMaterial>>() {
			app.add_plugins(MaterialPlugin::<StandardMaterial>::default());
		}
	}
}
