//! Asset tile: optional thumbnail plus a yellow label.

use bevy::prelude::*;
use bevy::text::{Justify, LineBreak, TextBounds};

use crate::theme::{
	PANEL_CHIP_GAP, PANEL_ITEM_FONT_SIZE, PANEL_TILE_MIN_HEIGHT, PANEL_TILE_MIN_WIDTH, TEXT_YELLOW,
	TEXT_YELLOW_FAINT, TEXT_YELLOW_HOVER,
};

use super::display::menu_display_name;
use super::HudFonts;

const TILE_LABEL_MAX_CHARS: usize = 16;

/// Short labels wrap; longer labels are elided.
pub fn tile_label(label: &str) -> String {
	if label.chars().count() <= TILE_LABEL_MAX_CHARS {
		return label.to_string();
	}
	let mut end = TILE_LABEL_MAX_CHARS.saturating_sub(1);
	while end > 0 && !label.is_char_boundary(end) {
		end -= 1;
	}
	format!("{}…", &label[..end])
}

/// Pickable asset cell. `extra` is typically `MenuButton<E>`.
pub fn spawn_asset_tile(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	selected: bool,
	thumbnail: Option<Handle<Image>>,
	muted: bool,
	extra: impl Bundle,
) {
	let face = tile_face(selected, muted);
	parent
		.spawn((
			Button,
			extra,
			Node {
				min_width: Val::Px(PANEL_TILE_MIN_WIDTH),
				min_height: Val::Px(PANEL_TILE_MIN_HEIGHT),
				padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
				flex_direction: FlexDirection::Column,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				row_gap: Val::Px(PANEL_CHIP_GAP),
				border: UiRect::all(Val::Px(if selected { 2.0 } else { 0.0 })),
				..default()
			},
			BorderColor::all(if selected { face } else { Color::NONE }),
			BackgroundColor(Color::NONE),
		))
		.with_children(|button| {
			if let Some(thumbnail) = thumbnail {
				button.spawn((
					ImageNode::new(thumbnail),
					Node { width: Val::Px(54.0), height: Val::Px(54.0), ..default() },
					Pickable::IGNORE,
				));
			}
			let bounds = (PANEL_TILE_MIN_WIDTH - 12.0).max(12.0);
			button.spawn((
				Text::new(tile_label(&menu_display_name(label))),
				fonts.item(PANEL_ITEM_FONT_SIZE),
				TextColor(face),
				TextLayout::new(Justify::Center, LineBreak::WordBoundary),
				TextBounds::new(bounds, bounds),
				Pickable::IGNORE,
			));
		});
}

/// 1-based rank in a typed inventory slot. Derived from bag selection order.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotRank(pub u8);

/// Pickable catalog cell. Clothing selection uses the Maybraid son; weapons
/// selection uses the 1-based queue rank.
pub fn spawn_grid_catalog_tile(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	selected: bool,
	rank: Option<u8>,
	thumbnail: Option<Handle<Image>>,
	muted: bool,
	extra: impl Bundle,
) {
	let face = tile_face(selected, muted);
	let mark = if muted { TEXT_YELLOW_FAINT } else { TEXT_YELLOW };
	let mut tile = parent.spawn((
		Button,
		extra,
		Node {
			min_width: Val::Px(PANEL_TILE_MIN_WIDTH),
			min_height: Val::Px(PANEL_TILE_MIN_HEIGHT),
			padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
			flex_direction: FlexDirection::Column,
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			row_gap: Val::Px(PANEL_CHIP_GAP),
			..default()
		},
		BackgroundColor(Color::NONE),
	));
	if let Some(rank) = rank {
		tile.insert(SlotRank(rank));
	}
	tile.with_children(|button| {
		button
			.spawn((
				Node {
					width: Val::Px(54.0),
					height: Val::Px(54.0),
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|slot| {
				if let Some(thumbnail) = thumbnail {
					slot.spawn((
						ImageNode::new(thumbnail),
						Node { width: Val::Px(54.0), height: Val::Px(54.0), ..default() },
						Pickable::IGNORE,
					));
				}
				if let Some(rank) = rank {
					slot.spawn((
						Text::new(rank.to_string()),
						fonts.item(PANEL_ITEM_FONT_SIZE),
						TextColor(mark),
						Pickable::IGNORE,
					));
				} else if selected {
					crate::icons::Icon::maybraid(22.0, mark).spawn(
						slot,
						fonts.logo.clone(),
						Visibility::Inherited,
					);
				}
			});
		let bounds = (PANEL_TILE_MIN_WIDTH - 12.0).max(12.0);
		button.spawn((
			Text::new(label),
			fonts.item(PANEL_ITEM_FONT_SIZE),
			TextColor(face),
			TextLayout::new(Justify::Center, LineBreak::WordBoundary),
			TextBounds::new(bounds, bounds * 1.6),
			Pickable::IGNORE,
		));
	});
}

fn tile_face(selected: bool, muted: bool) -> Color {
	if muted {
		TEXT_YELLOW_FAINT
	} else if selected {
		TEXT_YELLOW_HOVER
	} else {
		TEXT_YELLOW
	}
}

pub fn spawn_tile_grid(
	parent: &mut ChildSpawnerCommands,
	justify: JustifyContent,
	children: impl FnOnce(&mut ChildSpawnerCommands),
) {
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Row,
				flex_wrap: FlexWrap::Wrap,
				column_gap: Val::Px(PANEL_CHIP_GAP),
				row_gap: Val::Px(PANEL_CHIP_GAP),
				align_items: AlignItems::FlexStart,
				justify_content: justify,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(children);
}

#[cfg(test)]
mod tests {
	use super::tile_label;

	#[test]
	fn short_label_unchanged() {
		assert_eq!(tile_label("Hair"), "Hair");
	}

	#[test]
	fn long_label_elides() {
		let label = tile_label("extraordinarily-long-asset");
		assert!(label.ends_with('…'));
		assert!(label.chars().count() <= 16);
	}
}
