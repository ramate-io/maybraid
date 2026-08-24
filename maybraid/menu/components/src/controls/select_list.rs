//! Vertical text row with a cursor gutter on the selected item.

use bevy::prelude::*;

use crate::theme::{PANEL_CURSOR_ICON_GAP, TEXT_YELLOW, TEXT_YELLOW_HOVER};

use super::text::{spawn_cursor_slot, spawn_item_label};
use super::HudFonts;

/// One pickable list row. `extra` is typically `MenuButton<E>`.
pub fn spawn_select_row(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	selected: bool,
	justify: JustifyContent,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Button,
			extra,
			Node {
				width: Val::Percent(100.0),
				padding: UiRect::axes(Val::Px(0.0), Val::Px(2.0)),
				flex_direction: FlexDirection::Row,
				justify_content: justify,
				align_items: AlignItems::Center,
				column_gap: Val::Px(PANEL_CURSOR_ICON_GAP),
				..default()
			},
			BackgroundColor(Color::NONE),
		))
		.with_children(|row| {
			spawn_cursor_slot(row, fonts, selected);
			spawn_item_label(
				row,
				fonts,
				label,
				if selected { TEXT_YELLOW_HOVER } else { TEXT_YELLOW },
			);
		});
}
