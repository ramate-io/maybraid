//! Registers mesh dispatch for [`SimplemansHedge`](crate::simplemans_hedge::SimplemansHedge) [`ChicoBall`](chico_ball_components::chico_ball::ChicoBall) and [`PlaneSplay`](chico_ball_components::plane_splay::PlaneSplay) children.

use bevy::prelude::*;
use chico_ball_components::chico_ball::render_item_plugin::ChicoBallRenderItemPlugin;

pub struct SimplemansHedgeRenderItemPlugin;

impl Default for SimplemansHedgeRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<SimplemansHedgeRenderItemPlugin>() {
		return;
	}
	app.add_plugins(SimplemansHedgeRenderItemPlugin::default());
}

impl Plugin for SimplemansHedgeRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoBallRenderItemPlugin>() {
			app.add_plugins(ChicoBallRenderItemPlugin::default());
		}
	}
}
