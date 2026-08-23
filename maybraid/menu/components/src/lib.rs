//! Reusable Maybraid HUD widgets.
//!
//! Start here with a bottom-left text column; screens compose these into
//! plugins, and the playground iterates on look and behavior.

pub mod text_menu;
pub mod theme;

pub use text_menu::{
	activate_clicked_text_menu_items, activate_selected_text_menu_items, navigate_text_menus,
	spawn_text_menu_header, spawn_text_menu_item, sync_hover_to_text_menu_selection,
	sync_text_menu_item_colors, text_menu_column_node, TextMenu, TextMenuInputLock, TextMenuItem,
	TextMenuItemAction, TextMenuItemLabel,
};
pub use theme::{
	MenuFonts, BARLOW_BLACK, BARLOW_SEMIBOLD, HEADER_FONT_SIZE, ITEM_FONT_SIZE, TEXT_YELLOW,
	TEXT_YELLOW_HOVER,
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
		app.init_resource::<MenuFonts>()
			.init_resource::<TextMenuInputLock>()
			.configure_sets(Update, TextMenuSystems::InputLock.before(TextMenuSystems::Navigate))
			.add_systems(
				Update,
				(
					sync_hover_to_text_menu_selection,
					navigate_text_menus,
					sync_text_menu_item_colors,
				)
					.chain()
					.in_set(TextMenuSystems::Navigate),
			);
	}
}
