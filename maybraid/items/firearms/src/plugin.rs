//! Register nested firearm LodScene hosts plus fulfill plugins.

use bevy::prelude::*;
use firearms_components::{add_firearm_components_host, FirearmComponentsPlugin};
use lod::LodRefreshCorePlugin;
use scene_ref::SceneRefPlugin;

use crate::kit::FirearmKit;

/// Scene-ref, LOD refresh, socket fulfill, and the kit host.
///
/// Playgrounds that spawn [`crate::ComponentsOnly`] via [`lod::LodScene::host`]
/// add this once.
pub struct FirearmHostsPlugin;

impl Plugin for FirearmHostsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.add_plugins(SceneRefPlugin);
		}
		if !app.is_plugin_added::<LodRefreshCorePlugin>() {
			app.add_plugins(LodRefreshCorePlugin);
		}
		if !app.is_plugin_added::<FirearmComponentsPlugin>() {
			app.add_plugins(FirearmComponentsPlugin);
		}
		add_firearm_components_host::<FirearmKit>(app);
	}
}
