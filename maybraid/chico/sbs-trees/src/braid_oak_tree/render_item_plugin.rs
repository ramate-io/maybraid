//! Registers mesh dispatch for Braid Oak Tree sticks and inner-ball canopy.

use bevy::prelude::*;
use chico_ball_components::chico_ball::render_item_plugin::ChicoBallRenderItemPlugin;
use chico_stick_components::chico_crook_stick::render_item_plugin::ChicoCrookStickRenderItemPlugin;

pub struct BraidOakTreeRenderItemPlugin;

impl Default for BraidOakTreeRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<BraidOakTreeRenderItemPlugin>() {
		return;
	}
	app.add_plugins(BraidOakTreeRenderItemPlugin::default());
}

impl Plugin for BraidOakTreeRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoCrookStickRenderItemPlugin>() {
			app.add_plugins(ChicoCrookStickRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<ChicoBallRenderItemPlugin>() {
			app.add_plugins(ChicoBallRenderItemPlugin::default());
		}
	}
}
