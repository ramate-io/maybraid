//! Reusable Maybraid HUD widgets.
//!
//! Widgets expose [`Scene`] constructors (`bsn!`). Screens stamp an API-specific
//! choice component onto each pickable row; picking copies that component out
//! as a [`Message`].

pub mod description;
pub mod text_menu;
pub mod theme;

pub use description::TextMenuDescription;
pub use text_menu::{
	activate_selected_text_menu_items, emit_menu_choice, emit_menu_over,
	emit_menu_over_on_selection, navigate_text_menus, select_text_menu_item_on_over,
	sync_text_menu_item_colors, MenuOver, TextMenu, TextMenuColumn, TextMenuHeader,
	TextMenuInputLock, TextMenuItem, TextMenuItemLabel,
};
pub use theme::{
	BARLOW_BLACK, BARLOW_REGULAR, BARLOW_SEMIBOLD, HEADER_FONT_SIZE, ITEM_FONT_SIZE, TEXT_YELLOW,
	TEXT_YELLOW_FAINT, TEXT_YELLOW_HOVER,
};

use bevy::prelude::*;

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
