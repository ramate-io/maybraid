//! Single-choice HUD menus.

pub mod text_cursor;
pub mod text_menu;

pub use text_cursor::{
	consume_screen_back, emit_screen_back_on_click, screen_back_scene, sync_text_cursor_icons,
	ButtonWithSubtext, ScreenBack, ScreenBackPressed, TextCursorColumn, TextCursorMenu,
	TextCursorRow, TextCursorSlot,
};
pub use text_menu::{
	apply_text_menu_nav, emit_menu_activate_on_click, emit_menu_activate_on_enter,
	emit_menu_activate_on_nav, emit_menu_focus, navigate_text_menus, republish_menu_activate,
	select_text_menu_item_on_over, sync_text_menu_item_colors, KeyboardMenuNav, MenuActivate,
	MenuFocus, TextColumnAlign, TextColumnAnchor, TextMenu, TextMenuColumn, TextMenuHeader,
	TextMenuInputLock, TextMenuItem, TextMenuItemLabel,
};
