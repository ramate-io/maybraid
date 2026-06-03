//! Registers mesh dispatch for Friend's Conifer sticks ([`PlaneSplay`](chico_ball_components::plane_splay::PlaneSplay) spawns meshes directly).

use bevy::prelude::*;
use chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin;

pub struct FriendsConiferRenderItemPlugin;

impl Default for FriendsConiferRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<FriendsConiferRenderItemPlugin>() {
		return;
	}
	app.add_plugins(FriendsConiferRenderItemPlugin::default());
}

impl Plugin for FriendsConiferRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoStickRenderItemPlugin>() {
			app.add_plugins(ChicoStickRenderItemPlugin::default());
		}
	}
}
