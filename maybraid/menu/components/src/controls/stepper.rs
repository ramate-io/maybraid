//! `< value >` / `− n +` control strip.

use bevy::prelude::*;
use bevy::text::Justify;

use crate::theme::{PANEL_VALUE_FONT_SIZE, TEXT_YELLOW};

use super::button::spawn_text_button;
use super::text::spawn_hud_text;
use super::HudFonts;

/// Compact horizontal stepper. `minus` / `plus` are stamped onto the arrows.
pub fn spawn_stepper(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	minus_label: &str,
	plus_label: &str,
	value: &str,
	minus: impl Bundle,
	plus: impl Bundle,
) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(8.0),
				align_items: AlignItems::Center,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			spawn_text_button(row, fonts, minus_label, minus);
			spawn_hud_text(
				row,
				fonts.item(PANEL_VALUE_FONT_SIZE),
				value,
				TEXT_YELLOW,
				Justify::Center,
			);
			spawn_text_button(row, fonts, plus_label, plus);
		});
}
