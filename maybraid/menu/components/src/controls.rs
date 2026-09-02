//! Mid-size HUD controls for inspector-style panels.
//!
//! These are imperative spawn helpers for dynamic trees (character menus).
//! Full-screen home / loading screens keep using BSN [`Scene`]s.

pub mod button;
pub mod display;
pub mod fonts;
pub mod hud_menu;
pub mod row;
pub mod scroll;
pub mod section;
pub mod short_text;
pub mod stepper;
pub mod swatch;
pub mod text;
pub mod tile;

pub use button::{spawn_hud_action, spawn_text_button};
pub use display::menu_display_name;
pub use fonts::HudFonts;
pub use hud_menu::{
	apply_hud_menu_nav, navigate_hud_menus, select_hud_item_on_over, HudMenu, HudMenuItem,
	HudOverlayMenu,
};
pub use row::spawn_labeled_row;
pub use scroll::{
	on_hud_scroll, send_hud_scroll_events, spawn_scroll_pane, sync_hud_scrollbars, HudScroll,
	HudScrollThumb, HudScrollTrack, HudScrollViewport,
};
pub use section::{
	spawn_section_header, sync_overlay_header_cursors, ActiveOverlayKey, OverlayHeader,
	OverlayHeaderKey,
};
pub use short_text::{
	capture_short_text_input, emit_short_text_cancel_on_click, emit_short_text_pad_on_click,
	emit_short_text_submit_on_click, emit_short_text_toggle_on_click,
	emit_short_text_toggle_on_enter, emit_short_text_toggle_on_nav, restore_short_text_editing,
	spawn_short_text_button, sync_short_text_cursors, sync_short_text_display, sync_short_text_ime,
	sync_short_text_modal, ActiveShortText, ShortTextChange, ShortTextField, ShortTextKey,
	ShortTextModal, ShortTextToggle, ShortTextValue,
};
pub use stepper::spawn_stepper;
pub use swatch::{color_from_hex, spawn_swatch, spawn_swatch_row};
pub use text::{
	spawn_block_label, spawn_cursor_slot, spawn_cursor_slot_sized, spawn_group_label,
	spawn_header_line, spawn_hud_text, spawn_panel_title,
};
pub use tile::{spawn_asset_tile, spawn_grid_catalog_tile, spawn_tile_grid};
