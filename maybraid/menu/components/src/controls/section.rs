//! Overlay-opening section header.

use bevy::prelude::*;

use crate::icons::AnimatedIcon;
use crate::single_select::TextCursorSlot;
use crate::theme::{PANEL_CURSOR_ICON_GAP, PANEL_HEADER_CURSOR_ICON_SIZE, PANEL_HEADER_FONT_SIZE};

use super::hud_menu::{HudMenu, HudMenuItem};
use super::text::{spawn_cursor_slot_sized, spawn_header_line};
use super::HudFonts;

/// Marker on a header that opens an overlay select.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct OverlayHeader;

/// Label key for [`ActiveOverlayKey`] matching.
#[derive(Component, Debug, Clone, Copy)]
pub struct OverlayHeaderKey(pub &'static str);

/// Currently open overlay key; empty means the panel is idle.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ActiveOverlayKey(pub Option<&'static str>);

/// Pickable header. `extra` is typically `OpenSelectKey`.
///
/// The cursor starts hidden; [`sync_hud_cursors`] shows it when the row is
/// hovered, focused, or its overlay is open.
pub fn spawn_section_header(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &'static str,
	value: Option<&str>,
	justify: JustifyContent,
	color: Color,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Button,
			OverlayHeader,
			OverlayHeaderKey(label),
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
			spawn_header_line(row, fonts, label, value, PANEL_HEADER_FONT_SIZE, color);
		});
}

/// Marker on HUD rows that reserve a Maybraid select gutter.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CursorRow;

/// Wink the mark while a cursor row is hovered, focused, or its overlay is open.
pub fn sync_hud_cursors(
	active: Res<ActiveOverlayKey>,
	rows: Query<
		(&Children, Option<&HudMenuItem>, Option<&OverlayHeaderKey>, Option<&Interaction>),
		Or<(With<CursorRow>, With<OverlayHeader>)>,
	>,
	menus: Query<&HudMenu>,
	slots: Query<(), With<TextCursorSlot>>,
	children: Query<&Children>,
	mut icons: Query<&mut Visibility, With<AnimatedIcon>>,
) {
	for (row_children, item, key, interaction) in &rows {
		let focused = item
			.is_some_and(|item| menus.get(item.menu).is_ok_and(|menu| menu.selected == item.index));
		let overlay = key.is_some_and(|key| active.0 == Some(key.0));
		let hovered = matches!(interaction, Some(Interaction::Hovered | Interaction::Pressed));
		let show = focused || overlay || hovered;
		for child in row_children {
			if slots.get(*child).is_err() {
				continue;
			}
			let Ok(slot_children) = children.get(*child) else {
				continue;
			};
			for icon_entity in slot_children {
				if let Ok(mut visibility) = icons.get_mut(*icon_entity) {
					*visibility = if show { Visibility::Inherited } else { Visibility::Hidden };
				}
			}
		}
	}
}
