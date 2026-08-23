//! Reusable Maybraid HUD widgets.
//!
//! Widgets expose [`Scene`] constructors (`bsn!`). Screens stamp an API-specific
//! choice component onto each pickable row. Selection triggers [`MenuFocus`]
//! and click / Enter trigger [`MenuActivate`], both on the menu. The screen
//! observes those events; [`republish_menu_activate`] copies activate out as a
//! [`Message`].

pub mod description;
pub mod text_menu;
pub mod theme;

pub use description::{set_description_for_menu, TextMenuDescription};
pub use text_menu::{
	emit_menu_activate_on_click, emit_menu_activate_on_enter, emit_menu_focus, navigate_text_menus,
	republish_menu_activate, select_text_menu_item_on_over, sync_text_menu_item_colors,
	MenuActivate, MenuFocus, TextMenu, TextMenuColumn, TextMenuHeader, TextMenuInputLock,
	TextMenuItem, TextMenuItemLabel,
};
pub use theme::{
	BARLOW_BLACK, BARLOW_REGULAR, BARLOW_SEMIBOLD, HEADER_FONT_SIZE, ITEM_FONT_SIZE, TEXT_YELLOW,
	TEXT_YELLOW_FAINT, TEXT_YELLOW_HOVER,
};

use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextMenuSystems {
	InputLock,
	Navigate,
}

pub struct MenuComponentsPlugin;

impl Plugin for MenuComponentsPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<TextMenuInputLock>()
			.configure_sets(Update, TextMenuSystems::InputLock.before(TextMenuSystems::Navigate))
			.add_observer(select_text_menu_item_on_over)
			.add_systems(
				Update,
				(navigate_text_menus, sync_text_menu_item_colors)
					.chain()
					.in_set(TextMenuSystems::Navigate),
			);
	}
}

/// Per-choice-type wiring: focus / activate emit, and [`Message<E>`] registration.
pub struct TextMenuPlugin<E>(PhantomData<fn() -> E>);

impl<E> Default for TextMenuPlugin<E> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<E> Plugin for TextMenuPlugin<E>
where
	E: Message + Component + Copy + Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		app.add_message::<E>()
			.add_observer(emit_menu_activate_on_click::<E>)
			.add_systems(
				Update,
				(
					emit_menu_focus::<E>.after(TextMenuSystems::Navigate),
					emit_menu_activate_on_enter::<E>,
				),
			);
	}
}
