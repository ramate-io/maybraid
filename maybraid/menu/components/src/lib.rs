//! Reusable Maybraid HUD widgets.
//!
//! Widgets expose [`Scene`] constructors (`bsn!`). Screens stamp an API-specific
//! choice component onto each pickable row. Selection triggers [`MenuFocus`]
//! and click / Enter trigger [`MenuActivate`], both on the menu. The screen
//! observes those events; [`republish_menu_activate`] copies activate out as a
//! [`Message`].

pub mod controls;
pub mod icons;
pub mod info;
pub mod loading;
pub mod single_select;
pub mod theme;

pub use controls::{
	ActiveOverlayKey, ActiveShortText, HudFonts, HudMenu, HudMenuItem, HudOverlayMenu, HudScroll,
	HudScrollThumb, HudScrollTrack, HudScrollViewport, OverlayHeader, OverlayHeaderKey,
	ShortTextChange, ShortTextField, ShortTextKey, ShortTextModal, ShortTextToggle, ShortTextValue,
	color_from_hex, menu_display_name, navigate_hud_menus, on_hud_scroll,
	restore_short_text_editing, select_hud_item_on_over, send_hud_scroll_events, spawn_asset_tile,
	spawn_block_label, spawn_cursor_slot, spawn_cursor_slot_sized, spawn_group_label,
	spawn_header_line, spawn_hud_text, spawn_labeled_row, spawn_panel_title, spawn_scroll_pane,
	spawn_section_header, spawn_short_text_button, spawn_stepper, spawn_swatch, spawn_swatch_row,
	spawn_text_button, spawn_tile_grid, sync_hud_scrollbars, sync_overlay_header_cursors,
};
pub use icons::{AnimatedIcon, Icon, SpinningIcon, blink_animated_icons, spin_icons};
pub use info::{
	TextMenuDescription, TextMenuHint, TextMenuHintLabel, set_description_for_menu,
	set_hint_for_menu,
};
pub use loading::{
	LoadingBarFill, LoadingExplainer, LoadingPanel, LoadingStack, set_loading_explainer,
	set_loading_progress, sync_loading_bar_fill,
};
pub use single_select::{
	MenuActivate, MenuFocus, TextCursorColumn, TextCursorMenu, TextCursorSlot, TextMenu,
	TextMenuColumn, TextMenuHeader, TextMenuInputLock, TextMenuItem, TextMenuItemLabel,
	emit_menu_activate_on_click, emit_menu_activate_on_enter, emit_menu_focus, navigate_text_menus,
	republish_menu_activate, select_text_menu_item_on_over, sync_text_cursor_icons,
	sync_text_menu_item_colors,
};
pub use theme::{
	BARLOW_BLACK, BARLOW_REGULAR, BARLOW_SEMIBOLD, HEADER_FONT_SIZE, ITEM_FONT_SIZE,
	PANEL_HEADER_FONT_SIZE, PANEL_ITEM_FONT_SIZE, PANEL_LABEL_FONT_SIZE, PANEL_ROW_GAP,
	TEXT_YELLOW, TEXT_YELLOW_FAINT, TEXT_YELLOW_HOVER,
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
			.init_resource::<ActiveOverlayKey>()
			.init_resource::<ActiveShortText>()
			.init_resource::<ShortTextModal>()
			.configure_sets(Update, TextMenuSystems::InputLock.before(TextMenuSystems::Navigate))
			.add_observer(select_text_menu_item_on_over)
			.add_observer(select_hud_item_on_over)
			.add_observer(on_hud_scroll)
			.add_observer(controls::emit_short_text_toggle_on_click)
			.add_observer(controls::emit_short_text_submit_on_click)
			.add_observer(controls::emit_short_text_pad_on_click)
			.add_observer(controls::emit_short_text_cancel_on_click)
			.add_systems(
				Update,
				(
					blink_animated_icons,
					spin_icons,
					sync_loading_bar_fill,
					send_hud_scroll_events,
					sync_hud_scrollbars,
					sync_overlay_header_cursors,
					controls::restore_short_text_editing,
					controls::sync_short_text_display,
					controls::sync_short_text_cursors,
					controls::sync_short_text_ime,
					controls::sync_short_text_modal,
					controls::short_text::sync_short_text_pad_shift,
					controls::emit_short_text_toggle_on_enter,
					controls::capture_short_text_input,
				),
			)
			.add_systems(
				Update,
				(
					navigate_text_menus,
					navigate_hud_menus,
					sync_text_menu_item_colors,
					sync_text_cursor_icons,
				)
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
