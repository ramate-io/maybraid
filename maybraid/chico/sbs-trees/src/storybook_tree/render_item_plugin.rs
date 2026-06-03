//! Registers mesh dispatch for Storybook Tree sticks and plane-splay canopy.

use bevy::prelude::*;
use chico_stick_components::chico_stick::render_item_plugin::ChicoStickRenderItemPlugin;

pub struct StorybookTreeRenderItemPlugin;

impl Default for StorybookTreeRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<StorybookTreeRenderItemPlugin>() {
		return;
	}
	app.add_plugins(StorybookTreeRenderItemPlugin::default());
}

impl Plugin for StorybookTreeRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ChicoStickRenderItemPlugin>() {
			app.add_plugins(ChicoStickRenderItemPlugin::default());
		}
	}
}
