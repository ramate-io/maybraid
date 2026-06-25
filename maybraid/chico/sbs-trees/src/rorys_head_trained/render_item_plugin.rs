//! Registers mesh dispatch for Rory's Head-trained crook sticks and leaf balls.

use bevy::prelude::*;
use chico_ball_components::chico_ball::render_item_plugin::ChicoBallRenderItemPlugin;
use chico_stick_components::chico_crook_stick::render_item_plugin::ChicoCrookStickRenderItemPlugin;

pub struct RorysHeadTrainedRenderItemPlugin;

impl Default for RorysHeadTrainedRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<RorysHeadTrainedRenderItemPlugin>() {
		return;
	}
	app.add_plugins(RorysHeadTrainedRenderItemPlugin::default());
}

impl Plugin for RorysHeadTrainedRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoCrookStickRenderItemPlugin>() {
			app.add_plugins(ChicoCrookStickRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<ChicoBallRenderItemPlugin>() {
			app.add_plugins(ChicoBallRenderItemPlugin::default());
		}
	}
}
