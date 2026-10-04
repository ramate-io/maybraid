//! Single-choice HUD menus.

pub mod text_cursor;
pub mod text_menu;

pub use text_cursor::{
	ButtonWithSubtext, MenuBackConsumed, MenuObjectiveKind, MenuObjectiveMarker, ScreenBack,
	ScreenBackPressed, ScreenEdit, ScreenEditPressed, TextCursorColumn, TextCursorMenu,
	TextCursorRow, TextCursorScroll, TextCursorSlot, clear_menu_back_consumed, consume_screen_back,
	emit_screen_back_on_click, emit_screen_edit_on_click, screen_back_scene, screen_edit_scene,
	scroll_text_cursor_selection_into_view, sync_screen_edit_cursor, sync_text_cursor_icons,
};
pub use text_menu::{
	KeyboardMenuNav, MenuActivate, MenuFocus, MenuItemLocked, TextColumnAlign, TextColumnAnchor,
	TextMenu, TextMenuColumn, TextMenuHeader, TextMenuInputLock, TextMenuItem, TextMenuItemLabel,
	apply_text_menu_nav, emit_menu_activate_on_click, emit_menu_activate_on_enter,
	emit_menu_activate_on_nav, emit_menu_focus, navigate_text_menus, republish_menu_activate,
	select_text_menu_item_on_over, sync_text_menu_item_colors,
};
