//! Asset tile: optional thumbnail plus a yellow label.

use bevy::prelude::*;
use bevy::text::{Justify, LineBreak, LineHeight, TextBounds, TextSpan};

use crate::theme::{
	PANEL_CHIP_GAP, PANEL_GROUP_FONT_SIZE, PANEL_ITEM_FONT_SIZE, PANEL_TILE_MIN_HEIGHT,
	PANEL_TILE_MIN_WIDTH, TEXT_LIME, TEXT_SALMON, TEXT_YELLOW, TEXT_YELLOW_FAINT,
	TEXT_YELLOW_HOVER,
};

use super::display::menu_display_name;
use super::hud_menu::{HudMenu, HudMenuItem};
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
			HoverTile { equipped: selected, preserve_fill: false },
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
/// selection uses the 1-based queue rank. `detail` is a compact stat line.
pub fn spawn_grid_catalog_tile(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	detail: &str,
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
		HoverTile { equipped: selected, preserve_fill: false },
		extra,
		Node {
			min_width: Val::Px(PANEL_TILE_MIN_WIDTH),
			min_height: Val::Px(PANEL_TILE_MIN_HEIGHT),
			padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
			flex_direction: FlexDirection::Column,
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			row_gap: Val::Px(PANEL_CHIP_GAP),
			border: UiRect::all(Val::Px(0.0)),
			..default()
		},
		BorderColor::all(Color::NONE),
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
		if !detail.is_empty() {
			spawn_detail_line(button, fonts, detail, bounds);
		}
	});
}

fn spawn_detail_line(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	detail: &str,
	bounds: f32,
) {
	let mut parts = detail.split(" · ");
	let Some(first) = parts.next() else {
		return;
	};
	parent
		.spawn((
			Text::new(first),
			fonts.item(PANEL_GROUP_FONT_SIZE),
			TextColor(detail_chip_color(first)),
			TextLayout::new(Justify::Center, LineBreak::WordBoundary),
			TextBounds::new(bounds, bounds),
			LineHeight::RelativeToFont(1.0),
			Pickable::IGNORE,
		))
		.with_children(|text| {
			for part in parts {
				text.spawn((
					TextSpan::new(" · "),
					fonts.item(PANEL_GROUP_FONT_SIZE),
					TextColor(TEXT_YELLOW_FAINT),
					LineHeight::RelativeToFont(1.0),
				));
				text.spawn((
					TextSpan::new(part),
					fonts.item(PANEL_GROUP_FONT_SIZE),
					TextColor(detail_chip_color(part)),
					LineHeight::RelativeToFont(1.0),
				));
			}
		});
}

fn detail_chip_color(part: &str) -> Color {
	let trimmed = part.trim();
	if trimmed.starts_with('+') {
		TEXT_LIME
	} else if trimmed.starts_with('-') {
		TEXT_SALMON
	} else {
		TEXT_YELLOW_FAINT
	}
}

/// Inventory / catalog cell that pulses while focused or pointer-hovered.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoverTile {
	pub equipped: bool,
	/// Keep the authored fill (color chips). Tiles wash; swatches do not.
	pub preserve_fill: bool,
}

/// Pulse the focused or hovered tile so stick and mouse share the same cue.
pub fn sync_hover_tiles(
	time: Res<Time>,
	menus: Query<&HudMenu>,
	mut tiles: Query<(
		&HoverTile,
		Option<&HudMenuItem>,
		Option<&Interaction>,
		&mut Node,
		&mut BorderColor,
		&mut BackgroundColor,
	)>,
) {
	let pulse = 0.55 + 0.45 * (time.elapsed_secs() * 7.0).sin();
	for (tile, item, interaction, mut node, mut border, mut background) in &mut tiles {
		let focused = item
			.is_some_and(|item| menus.get(item.menu).is_ok_and(|menu| menu.selected == item.index));
		let hovered = matches!(interaction, Some(Interaction::Hovered | Interaction::Pressed));
		if focused || hovered {
			node.border = UiRect::all(Val::Px(2.0));
			*border = BorderColor::all(TEXT_YELLOW.with_alpha(0.4 + 0.6 * pulse));
			if !tile.preserve_fill {
				background.0 = Color::srgba(0.98, 0.86, 0.32, 0.06 + 0.12 * pulse);
			}
			continue;
		}
		if tile.equipped {
			node.border = UiRect::all(Val::Px(2.0));
			*border = BorderColor::all(TEXT_YELLOW_HOVER);
			if !tile.preserve_fill {
				background.0 = Color::NONE;
			}
			continue;
		}
		node.border = UiRect::all(Val::Px(if tile.preserve_fill { 1.0 } else { 0.0 }));
		*border = BorderColor::all(if tile.preserve_fill { TEXT_YELLOW_FAINT } else { Color::NONE });
		if !tile.preserve_fill {
			background.0 = Color::NONE;
		}
	}
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
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;

	use super::{sync_hover_tiles, tile_label, HoverTile};
	use crate::controls::hud_menu::{HudMenu, HudMenuItem};

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

	#[test]
	fn focused_tile_gains_a_border() {
		let mut world = World::new();
		world.init_resource::<Time>();
		let menu = world.spawn(HudMenu { selected: 1, item_count: 2 }).id();
		let idle = world
			.spawn((
				HoverTile { equipped: false, preserve_fill: false },
				HudMenuItem { index: 0, menu },
				Node::default(),
				BorderColor::all(Color::NONE),
				BackgroundColor(Color::NONE),
			))
			.id();
		let focused = world
			.spawn((
				HoverTile { equipped: false, preserve_fill: false },
				HudMenuItem { index: 1, menu },
				Node::default(),
				BorderColor::all(Color::NONE),
				BackgroundColor(Color::NONE),
			))
			.id();
		world.run_system_once(sync_hover_tiles).expect("hover");
		let idle_border = world.get::<BorderColor>(idle).expect("idle").top;
		let live_border = world.get::<BorderColor>(focused).expect("focused").top;
		assert_eq!(idle_border, Color::NONE);
		assert_ne!(live_border, Color::NONE);
	}
}
