//! Menu controller: pad nav → one focused menu in a screen subtree.

pub mod controller;
pub mod dispatch;

pub use controller::MenuController;

use bevy::prelude::*;
use maybraid_input::{PadGameplayEnabled, VirtualPadPlugin, VirtualPadSystems};
use menu_components::{KeyboardMenuNav, MenuComponentsPlugin, TextMenuInputLock, TextMenuSystems};

pub struct MenuControllerPlugin;

impl Plugin for MenuControllerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		app.insert_resource(KeyboardMenuNav(false))
			.add_systems(PreUpdate, sync_pad_from_menu_lock.before(VirtualPadSystems::Produce));
		app.add_systems(
			Update,
			(dispatch::refresh_menu_focus, dispatch::dispatch_menu_nav)
				.chain()
				.in_set(TextMenuSystems::Navigate),
		);
	}
}

fn sync_pad_from_menu_lock(lock: Res<TextMenuInputLock>, mut enabled: ResMut<PadGameplayEnabled>) {
	enabled.0 = !lock.0;
}
