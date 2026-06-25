//! Registers mesh dispatch for Date Palm sticks, fronds, and crown tuft.

use bevy::prelude::*;
use chico_ball_components::frond::FrondRenderItemPlugin;
use chico_ball_components::tuft::render_item_plugin::TuftRenderItemPlugin;
use chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin;

pub struct DatePalmRenderItemPlugin;

impl Default for DatePalmRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<DatePalmRenderItemPlugin>() {
		return;
	}
	app.add_plugins(DatePalmRenderItemPlugin::default());
}

impl Plugin for DatePalmRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoStickRenderItemPlugin>() {
			app.add_plugins(ChicoStickRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<FrondRenderItemPlugin>() {
			app.add_plugins(FrondRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<TuftRenderItemPlugin>() {
			app.add_plugins(TuftRenderItemPlugin::default());
		}
	}
}
