//! Registers mesh dispatch for Northern Conifer sticks, apex [`ChicoBall`](chico_ball_components::chico_ball::ChicoBall), and [`PlaneSplay`](chico_ball_components::plane_splay::PlaneSplay).

use bevy::prelude::*;
use chico_ball_components::chico_ball::render_item_plugin::ChicoBallRenderItemPlugin;
use chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin;

pub struct NorthernConiferRenderItemPlugin;

impl Default for NorthernConiferRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<NorthernConiferRenderItemPlugin>() {
		return;
	}
	app.add_plugins(NorthernConiferRenderItemPlugin::default());
}

impl Plugin for NorthernConiferRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoStickRenderItemPlugin>() {
			app.add_plugins(ChicoStickRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<ChicoBallRenderItemPlugin>() {
			app.add_plugins(ChicoBallRenderItemPlugin::default());
		}
	}
}
