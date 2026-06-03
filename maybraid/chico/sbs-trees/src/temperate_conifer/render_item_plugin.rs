//! Registers mesh dispatch for Temperate Conifer sticks and fronds.

use bevy::prelude::*;
use chico_ball_components::frond::FrondRenderItemPlugin;
use chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin;

pub struct TemperateConiferRenderItemPlugin;

impl Default for TemperateConiferRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<TemperateConiferRenderItemPlugin>() {
		return;
	}
	app.add_plugins(TemperateConiferRenderItemPlugin::default());
}

impl Plugin for TemperateConiferRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoStickRenderItemPlugin>() {
			app.add_plugins(ChicoStickRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<FrondRenderItemPlugin>() {
			app.add_plugins(FrondRenderItemPlugin::default());
		}
	}
}
