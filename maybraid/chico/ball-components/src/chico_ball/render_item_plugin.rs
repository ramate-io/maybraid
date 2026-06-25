//! Registers enforced-caching mesh dispatch for [`NoisyBall`](chico_sdf::NoisyBall) used by [`super::ChicoBall`].

use bevy::prelude::*;
use chico_sdf::NoisyBall;
use render_item::mesh::handle::EnforceCachingPlugin;

pub struct ChicoBallRenderItemPlugin;

impl Default for ChicoBallRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

impl Plugin for ChicoBallRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MaterialPlugin<StandardMaterial>>() {
			app.add_plugins(MaterialPlugin::<StandardMaterial>::default());
		}
		if !app.is_plugin_added::<EnforceCachingPlugin<NoisyBall, StandardMaterial>>() {
			app.add_plugins(EnforceCachingPlugin::<NoisyBall, StandardMaterial>::default());
		}
	}
}
