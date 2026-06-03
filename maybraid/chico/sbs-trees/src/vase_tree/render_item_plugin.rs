//! Registers mesh dispatch for Vase Tree sticks and canopy foliage.

use bevy::prelude::*;
use chico_ball_components::chico_ball::render_item_plugin::ChicoBallRenderItemPlugin;
use chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin;

pub struct VaseTreeRenderItemPlugin;

impl Default for VaseTreeRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<VaseTreeRenderItemPlugin>() {
		return;
	}
	app.add_plugins(VaseTreeRenderItemPlugin::default());
}

impl Plugin for VaseTreeRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoStickRenderItemPlugin>() {
			app.add_plugins(ChicoStickRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<ChicoBallRenderItemPlugin>() {
			app.add_plugins(ChicoBallRenderItemPlugin::default());
		}
	}
}
