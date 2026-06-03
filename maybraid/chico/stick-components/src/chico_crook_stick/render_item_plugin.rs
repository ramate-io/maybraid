//! Registers enforced-caching mesh dispatch for [`CrookCylinder`](chico_sdf::CrookCylinder) used by [`super::ChicoCrookStick`].

use bevy::prelude::*;
use chico_sdf::CrookCylinder;
use render_item::mesh::handle::EnforceCachingPlugin;

pub struct ChicoCrookStickRenderItemPlugin;

impl Default for ChicoCrookStickRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

impl Plugin for ChicoCrookStickRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MaterialPlugin<StandardMaterial>>() {
			app.add_plugins(MaterialPlugin::<StandardMaterial>::default());
		}
		if !app.is_plugin_added::<EnforceCachingPlugin<CrookCylinder, StandardMaterial>>() {
			app.add_plugins(EnforceCachingPlugin::<CrookCylinder, StandardMaterial>::default());
		}
	}
}
