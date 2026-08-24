//! Collapsible section title.

use bevy::prelude::*;

use crate::theme::{PANEL_CURSOR_ICON_GAP, TEXT_YELLOW, TEXT_YELLOW_HOVER};

use super::text::{spawn_cursor_slot, spawn_item_label};
use super::HudFonts;

/// Pickable section header. `extra` is typically `ToggleSectionKey`.
pub fn spawn_section_header(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	open: bool,
	justify: JustifyContent,
	extra: impl Bundle,
) {
	let title = format!("{} {label}", if open { "v" } else { ">" });
	parent
		.spawn((
			Button,
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
			spawn_cursor_slot(row, fonts, open);
			spawn_item_label(
				row,
				fonts,
				&title,
				if open { TEXT_YELLOW } else { TEXT_YELLOW_HOVER },
			);
		});
}
