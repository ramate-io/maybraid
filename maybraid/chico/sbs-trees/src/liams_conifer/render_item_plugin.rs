//! Registers mesh dispatch for Liam's Conifer [`ChicoStick`](chico_stick_components::chico_stick::ChicoStick) render items.
//! Tuft foliage will register here once `chico-ball-components` exposes tufts ([#244](https://github.com/ramate-io/maybraid/issues/244)).

use bevy::prelude::*;
use chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin;

pub struct LiamsConiferRenderItemPlugin;

impl Default for LiamsConiferRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

impl Plugin for LiamsConiferRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoStickRenderItemPlugin>() {
			app.add_plugins(ChicoStickRenderItemPlugin::default());
		}
	}
}
