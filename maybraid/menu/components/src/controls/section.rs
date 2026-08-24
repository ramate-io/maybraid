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
/// `value` is the selected name when this header is a single catalog pick.
/// The cursor starts hidden; [`sync_overlay_header_cursors`] shows it when
/// the row is hovered or its overlay is open.
pub fn spawn_section_header(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &'static str,
	value: Option<&str>,
	justify: JustifyContent,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Button,
			OverlayHeader,
			OverlayHeaderKey(label),
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
			spawn_header_line(row, fonts, label, value, PANEL_HEADER_FONT_SIZE);
		});
}

/// Wink the header mark while that row is focused or its overlay is open.
pub fn sync_overlay_header_cursors(
	active: Res<ActiveOverlayKey>,
	headers: Query<(&OverlayHeaderKey, Option<&HudMenuItem>, &Children), With<OverlayHeader>>,
	menus: Query<&HudMenu>,
	slots: Query<(), With<TextCursorSlot>>,
	children: Query<&Children>,
	mut icons: Query<&mut Visibility, With<AnimatedIcon>>,
) {
	for (key, item, header_children) in &headers {
		let focused = item
			.is_some_and(|item| menus.get(item.menu).is_ok_and(|menu| menu.selected == item.index));
		let show = focused || active.0 == Some(key.0);
		for child in header_children {
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
