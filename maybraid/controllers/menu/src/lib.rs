//! Menu controller: pad nav → one focused menu in a screen subtree.

pub mod controller;
pub mod dispatch;
pub mod primary;
pub mod repeat;
pub mod text;
pub use repeat::MenuRepeatConfig;

pub use controller::{MenuController, MenuStartPolicy};
pub use dispatch::{ExclusiveNav, MenuNavCapture, exclusive_nav_menu};
pub use maybraid_controllers_core::{
	CaptureKind, CaptureToken, InputRouter, InputRouting, InputSource, RoutingEvent, ScopeOwner,
	UiTarget,
};
pub use menu_components::{MenuAction, MenuActionImpulse, MenuSession};

use bevy::prelude::*;
use maybraid_controllers_core::{CaptureResolveSystems, ControllersCorePlugin};
use maybraid_virtual_pad_controller::{VirtualPadPlugin, VirtualPadSystems};
use menu_components::{KeyboardMenuNav, MenuComponentsPlugin, TextMenuSystems};

pub struct MenuControllerPlugin;

impl Plugin for MenuControllerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ControllersCorePlugin>() {
			app.add_plugins(ControllersCorePlugin);
		}
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		app.init_resource::<MenuNavCapture>()
			.add_observer(dispatch::interpret_menu_pad)
			.add_systems(
				PreUpdate,
				(
					dispatch::ensure_scoped_menu_sessions,
					dispatch::begin_scoped_menu_frame,
					text::mark_scoped_subtrees,
				)
					.chain(),
			)
			.add_systems(PostUpdate, text::sync_scoped_text_ui)
			.add_systems(
				Update,
				primary::clear_primary_menu_frame
					.before(maybraid_controllers_core::PadDeliverySystems),
			)
			.insert_resource(KeyboardMenuNav(false))
			.add_systems(
				PreUpdate,
				(primary::sync_primary_menu_host, primary::sync_text_menu_capture)
					.chain()
					.before(maybraid_controllers_core::PadPolicySystems)
					.before(VirtualPadSystems::Produce),
			);
		app.configure_sets(
			Update,
			(
				TextMenuSystems::Navigate.after(maybraid_controllers_core::PadDeliverySystems),
				menu_components::PrimaryMenuFrameSystems
					.after(maybraid_controllers_core::PadDeliverySystems),
			),
		);
		app.add_systems(
			Update,
			(primary::refresh_primary_menu_focus, dispatch::refresh_scoped_menu_focus)
				.chain()
				.in_set(CaptureResolveSystems)
				.after(menu_components::MenuModalSystems::Sync)
				.before(TextMenuSystems::Navigate),
		);
	}
}

#[cfg(test)]
mod tests;
