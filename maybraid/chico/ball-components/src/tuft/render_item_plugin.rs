//! Registers enforced-caching mesh dispatch for [`super::TuftCluster`] used by [`super::ChicoTuft`].

use bevy::prelude::*;
use render_item::mesh::handle::EnforceCachingPlugin;

use super::TuftCluster;

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
		if !app.is_plugin_added::<EnforceCachingPlugin<TuftCluster, StandardMaterial>>() {
			app.add_plugins(EnforceCachingPlugin::<TuftCluster, StandardMaterial>::default());
		}
	}
}
