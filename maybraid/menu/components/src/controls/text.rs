//! Labels, body copy, and the reserved cursor gutter.

use bevy::prelude::*;
use bevy::text::Justify;

use crate::icons::maybraid::AnimatedIcon;
use crate::single_select::TextCursorSlot;
use crate::theme::{
	PANEL_BLOCK_FONT_SIZE, PANEL_CURSOR_ICON_SIZE, PANEL_GROUP_FONT_SIZE, PANEL_HEADER_FONT_SIZE,
	PANEL_ITEM_FONT_SIZE, TEXT_YELLOW, TEXT_YELLOW_FAINT,
};

use super::HudFonts;

/// Largest panel title: section chrome and overlay picker headers.
pub fn spawn_panel_title(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, label: &str) {
	spawn_hud_text(parent, fonts.header(PANEL_HEADER_FONT_SIZE), label, TEXT_YELLOW, Justify::Left);
}

/// Block title above a field group (`Eyes`, `Hair`).
pub fn spawn_block_label(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, label: &str) {
	spawn_hud_text(parent, fonts.header(PANEL_BLOCK_FONT_SIZE), label, TEXT_YELLOW, Justify::Left);
}

/// Muted subheading above a grouped select list.
pub fn spawn_group_label(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, label: &str) {
	spawn_hud_text(
		parent,
		fonts.body(PANEL_GROUP_FONT_SIZE),
		label,
		TEXT_YELLOW_FAINT,
		Justify::Left,
	);
}

pub fn spawn_hud_text(
	parent: &mut ChildSpawnerCommands,
	font: TextFont,
	value: &str,
	color: Color,
	justify: Justify,
) {
	parent.spawn((
		Text::new(value.to_string()),
		font,
		TextColor(color),
		TextLayout::new(justify, bevy::text::LineBreak::WordBoundary),
		Pickable::IGNORE,
	));
}

/// Reserved gutter; the blinking mark is hidden unless `visible`.
pub fn spawn_cursor_slot(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, visible: bool) {
	spawn_cursor_slot_sized(parent, fonts, visible, PANEL_CURSOR_ICON_SIZE);
}

/// Cursor gutter sized to sit against a header or a field row.
pub fn spawn_cursor_slot_sized(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	visible: bool,
	size: f32,
) {
	let visibility = if visible { Visibility::Inherited } else { Visibility::Hidden };
	parent
		.spawn((
			TextCursorSlot,
			Node { width: Val::Px(size), height: Val::Px(size), flex_shrink: 0.0, ..default() },
			Pickable::IGNORE,
		))
		.with_children(|slot| {
			let (icon, animated) = AnimatedIcon::maybraid(size, TEXT_YELLOW);
			AnimatedIcon::spawn(icon, animated, slot, fonts.logo.clone(), visibility);
		});
}

pub fn spawn_item_label(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	color: Color,
) {
	spawn_hud_text(parent, fonts.item(PANEL_ITEM_FONT_SIZE), label, color, Justify::Left);
}
