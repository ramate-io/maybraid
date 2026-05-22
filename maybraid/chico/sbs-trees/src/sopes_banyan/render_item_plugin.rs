//! Registers mesh dispatch for Sope's Banyan [`ChicoStick`](chico_stick_components::chico_stick::ChicoStick) and [`ChicoBall`](chico_ball_components::chico_ball::ChicoBall) render items. [`PlaneSplay`](chico_ball_components::plane_splay::PlaneSplay) terminals spawn [`Mesh3d`](bevy::prelude::Mesh3d) children directly and do not use the noisy-ball cache.

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
		if !app.is_plugin_added::<ChicoStickRenderItemPlugin>() {
			app.add_plugins(ChicoStickRenderItemPlugin::default());
		}
		if !app.is_plugin_added::<ChicoBallRenderItemPlugin>() {
			app.add_plugins(ChicoBallRenderItemPlugin::default());
		}
	}
}
