//! Registers enforced-caching mesh dispatch for [`NoisyCylinder`](chico_sdf::NoisyCylinder) used by [`super::ChicoStick`].

use bevy::prelude::*;
use chico_sdf::NoisyCylinder;
use render_item::mesh::handle::EnforceCachingPlugin;

pub struct ChicoStickRenderItemPlugin;

impl Default for ChicoStickRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

impl Plugin for ChicoStickRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MaterialPlugin<StandardMaterial>>() {
			app.add_plugins(MaterialPlugin::<StandardMaterial>::default());
		}
		if !app.is_plugin_added::<EnforceCachingPlugin<NoisyCylinder, StandardMaterial>>() {
			app.add_plugins(EnforceCachingPlugin::<NoisyCylinder, StandardMaterial>::default());
		}
	}
}
