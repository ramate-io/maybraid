//! Registers mesh dispatch for Kamakura Torch sticks and layered terminal canopy.

use bevy::prelude::*;
use chico_ball_components::chico_ball::render_item_plugin::ChicoBallRenderItemPlugin;
use chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin;

pub struct KamakuraTorchRenderItemPlugin;

impl Default for KamakuraTorchRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<KamakuraTorchRenderItemPlugin>() {
		return;
	}
	app.add_plugins(KamakuraTorchRenderItemPlugin::default());
}

impl Plugin for KamakuraTorchRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoStickRenderItemPlugin>() {
			app.add_plugins(ChicoStickRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<ChicoBallRenderItemPlugin>() {
			app.add_plugins(ChicoBallRenderItemPlugin::default());
		}
	}
}
