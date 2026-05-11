//! Registers mesh-dispatch plugins for Sope's Banyan stick and ball [`RenderItem`](render_item::RenderItem) types (see [`ChicoStickRenderItemPlugin`](chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin) / [`ChicoBallRenderItemPlugin`](chico_ball_components::chico_ball::render_item_plugin::ChicoBallRenderItemPlugin)).

use bevy::prelude::*;
use chico_ball_components::chico_ball::render_item_plugin::ChicoBallRenderItemPlugin;
use chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin;

pub struct SopesBanyanRenderItemPlugin;

impl Default for SopesBanyanRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

impl Plugin for SopesBanyanRenderItemPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ChicoStickRenderItemPlugin::default());
		app.add_plugins(ChicoBallRenderItemPlugin::default());
	}
}
