//! Summary row that opens an overlay select.

use bevy::prelude::*;

use crate::theme::{
	PANEL_BLOCK_FONT_SIZE, PANEL_CURSOR_ICON_GAP, PANEL_HEADER_CURSOR_ICON_SIZE,
	PANEL_ITEM_FONT_SIZE, TEXT_YELLOW, TEXT_YELLOW_FAINT,
};

use super::display::menu_display_name;
use super::text::{spawn_cursor_slot_sized, spawn_hud_text};
use super::HudFonts;

/// Pickable row: label, current value, cursor. `extra` is typically `OpenSelectKey`.
pub fn spawn_select_summary(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	value: &str,
	justify: JustifyContent,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Button,
			extra,
			Node {
				width: Val::Percent(100.0),
				padding: UiRect::axes(Val::Px(0.0), Val::Px(4.0)),
				flex_direction: FlexDirection::Row,
				justify_content: justify,
				align_items: AlignItems::FlexEnd,
				column_gap: Val::Px(PANEL_CURSOR_ICON_GAP),
				..default()
			},
			BackgroundColor(Color::NONE),
		))
		.with_children(|row| {
			spawn_cursor_slot_sized(row, fonts, true, PANEL_HEADER_CURSOR_ICON_SIZE);
			let title = menu_display_name(label);
			let value = menu_display_name(value);
			row.spawn((
				Node {
					flex_direction: FlexDirection::Row,
					column_gap: Val::Px(PANEL_CURSOR_ICON_GAP),
					align_items: AlignItems::Baseline,
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|pair| {
				spawn_hud_text(
					pair,
					fonts.header(PANEL_BLOCK_FONT_SIZE),
					&title,
					TEXT_YELLOW,
					bevy::text::Justify::Left,
				);
				spawn_hud_text(
					pair,
					fonts.body(PANEL_ITEM_FONT_SIZE),
					&value,
					TEXT_YELLOW_FAINT,
					bevy::text::Justify::Left,
				);
			});
		});
}
