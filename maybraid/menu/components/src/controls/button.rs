//! Transparent text button used by steppers and compact controls.

use bevy::prelude::*;
use bevy::text::Justify;

use crate::theme::{
	CURSOR_ICON_GAP, CURSOR_ICON_SIZE, ITEM_FONT_SIZE, PANEL_CURSOR_ICON_GAP,
	PANEL_HEADER_CURSOR_ICON_SIZE, PANEL_HEADER_FONT_SIZE, PANEL_VALUE_FONT_SIZE, TEXT_YELLOW,
};

use super::section::CursorRow;
use super::text::{spawn_cursor_slot_sized, spawn_header_line, spawn_hud_text};
use super::HudFonts;

/// Pickable label with no chip background. `extra` is typically `MenuButton<E>`.
pub fn spawn_text_button(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Button,
			extra,
			Node {
				min_width: Val::Px(22.0),
				padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(Color::NONE),
		))
		.with_children(|button| {
			spawn_hud_text(
				button,
				fonts.item(PANEL_VALUE_FONT_SIZE),
				label,
				TEXT_YELLOW,
				Justify::Center,
			);
		});
}

/// Full-width HUD action row (same mark + header type as section headers).
pub fn spawn_hud_action(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &'static str,
	justify: JustifyContent,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Button,
			CursorRow,
			extra,
			Node {
				width: Val::Percent(100.0),
				padding: UiRect::axes(Val::Px(0.0), Val::Px(4.0)),
				flex_direction: FlexDirection::Row,
				justify_content: justify,
				align_items: AlignItems::Center,
				column_gap: Val::Px(PANEL_CURSOR_ICON_GAP),
				..default()
			},
			BackgroundColor(Color::NONE),
		))
		.with_children(|row| {
			spawn_cursor_slot_sized(row, fonts, false, PANEL_HEADER_CURSOR_ICON_SIZE);
			spawn_header_line(row, fonts, label, None, PANEL_HEADER_FONT_SIZE, TEXT_YELLOW);
		});
}

/// Screen-chrome action (bottom-right Save Character). Same mark + face as
/// spin-reveal Next, without the caption.
pub fn spawn_corner_action(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Button,
			CursorRow,
			extra,
			Node {
				flex_direction: FlexDirection::Row,
				justify_content: JustifyContent::FlexStart,
				align_items: AlignItems::Center,
				column_gap: Val::Px(CURSOR_ICON_GAP),
				padding: UiRect::axes(Val::Px(0.0), Val::Px(2.0)),
				..default()
			},
			BackgroundColor(Color::NONE),
		))
		.with_children(|row| {
			spawn_cursor_slot_sized(row, fonts, false, CURSOR_ICON_SIZE);
			spawn_hud_text(row, fonts.item(ITEM_FONT_SIZE), label, TEXT_YELLOW, Justify::Left);
		});
}
