//! Registers mesh dispatch for Honu Banyan.

use bevy::prelude::*;
use chico_ball_components::chico_ball::render_item_plugin::ChicoBallRenderItemPlugin;
use chico_ball_components::tuft::render_item_plugin::TuftRenderItemPlugin;
use chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin;

pub struct HonuBanyanRenderItemPlugin;

impl Default for HonuBanyanRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<HonuBanyanRenderItemPlugin>() {
		return;
	}
	app.add_plugins(HonuBanyanRenderItemPlugin::default());
}

impl Plugin for HonuBanyanRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoStickRenderItemPlugin>() {
			app.add_plugins(ChicoStickRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<ChicoBallRenderItemPlugin>() {
			app.add_plugins(ChicoBallRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<TuftRenderItemPlugin>() {
			app.add_plugins(TuftRenderItemPlugin::default());
		}
	}
}
