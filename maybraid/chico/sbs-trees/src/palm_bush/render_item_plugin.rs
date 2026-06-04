//! Registers mesh dispatch for Palm Bush fronds and crown tuft.

use bevy::prelude::*;
use chico_ball_components::frond::FrondRenderItemPlugin;
use chico_ball_components::tuft::render_item_plugin::TuftRenderItemPlugin;

pub struct PalmBushRenderItemPlugin;

impl Default for PalmBushRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<PalmBushRenderItemPlugin>() {
		return;
	}
	app.add_plugins(PalmBushRenderItemPlugin::default());
}

impl Plugin for PalmBushRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<FrondRenderItemPlugin>() {
			app.add_plugins(FrondRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<TuftRenderItemPlugin>() {
			app.add_plugins(TuftRenderItemPlugin::default());
		}
	}
}
