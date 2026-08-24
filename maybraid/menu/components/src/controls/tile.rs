//! Asset tile: optional thumbnail plus a yellow label.

use bevy::prelude::*;
use bevy::text::{Justify, LineBreak, TextBounds};

use crate::theme::{
	PANEL_CHIP_GAP, PANEL_GROUP_FONT_SIZE, PANEL_TILE_MIN_HEIGHT, PANEL_TILE_MIN_WIDTH,
	TEXT_YELLOW, TEXT_YELLOW_HOVER,
};

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
	extra: impl Bundle,
) {
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
			BorderColor::all(if selected { TEXT_YELLOW } else { Color::NONE }),
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
				Text::new(tile_label(label)),
				fonts.item(PANEL_GROUP_FONT_SIZE),
				TextColor(if selected { TEXT_YELLOW_HOVER } else { TEXT_YELLOW }),
				TextLayout::new(Justify::Center, LineBreak::WordBoundary),
				TextBounds::new(bounds, bounds),
				Pickable::IGNORE,
			));
		});
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
