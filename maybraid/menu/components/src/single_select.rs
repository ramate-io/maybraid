//! Single-choice HUD menus.

pub mod text_cursor;
pub mod text_menu;

pub use text_cursor::{sync_text_cursor_icons, TextCursorColumn, TextCursorMenu, TextCursorSlot};
pub use text_menu::{
	emit_menu_activate_on_click, emit_menu_activate_on_enter, emit_menu_focus, navigate_text_menus,
	republish_menu_activate, select_text_menu_item_on_over, sync_text_menu_item_colors,
	MenuActivate, MenuFocus, TextColumnAlign, TextColumnAnchor, TextMenu, TextMenuColumn,
	TextMenuHeader, TextMenuInputLock, TextMenuItem, TextMenuItemLabel,
};
