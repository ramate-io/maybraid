//! Registers mesh dispatch for Liam's Conifer sticks and tufts.

use bevy::prelude::*;
use chico_ball_components::tuft::render_item_plugin::TuftRenderItemPlugin;
use chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin;

pub struct LiamsConiferRenderItemPlugin;

impl Default for LiamsConiferRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

/// Idempotent registration (safe when the playground and CLI both wire the same tree).
pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<LiamsConiferRenderItemPlugin>() {
		return;
	}
	app.add_plugins(LiamsConiferRenderItemPlugin::default());
}

impl Plugin for LiamsConiferRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoStickRenderItemPlugin>() {
			app.add_plugins(ChicoStickRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<TuftRenderItemPlugin>() {
			app.add_plugins(TuftRenderItemPlugin::default());
		}
	}
}
