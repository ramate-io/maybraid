//! Overlay routing for long catalogs. The IR stays `MenuNode`; this crate
//! decides which labels open a picker screen.

use bevy::prelude::*;
use character_ui_menu::{ItemRow, MenuNode, SelectGroup};

use crate::sink::{asset_thumbnail, bevy_color, MenuThumbnailContext, RenderContext};
use crate::widgets::{CloseOverlaySelect, MenuButton};
use crate::MenuJustify;
use menu_components::{
	spawn_asset_tile, spawn_block_label, spawn_group_label, spawn_select_summary, spawn_swatch,
	spawn_swatch_row, spawn_text_button, spawn_tile_grid, HudFonts, PANEL_ROW_GAP,
};

/// Labels that paint a summary row and open a full-grid picker.
pub fn overlay_select_label(label: &'static str) -> bool {
	matches!(label, "Species" | "Clothing" | "Animation" | "Clip")
}

/// Single-select overlays close after a pick; clothing stays open.
pub fn overlay_closes_on_pick(label: &'static str) -> bool {
	label != "Clothing"
}

/// Walk a normalized tree for the node whose label matches `key`.
pub fn find_overlay_node<'a, E>(nodes: &'a [MenuNode<E>], key: &str) -> Option<&'a MenuNode<E>> {
	for node in nodes {
		if let Some(found) = find_in_node(node, key) {
			return Some(found);
		}
	}
	None
}

fn find_in_node<'a, E>(node: &'a MenuNode<E>, key: &str) -> Option<&'a MenuNode<E>> {
	match node {
		MenuNode::Fragment(children) => find_overlay_node(children, key),
		MenuNode::Section { children, .. } => find_overlay_node(children, key),
		MenuNode::SectionSelect { label, children, .. } => {
			if *label == key {
				Some(node)
			} else {
				find_overlay_node(children, key)
			}
		}
		MenuNode::BlockAsset { label, .. } | MenuNode::ItemMultiSelect { label, .. }
			if *label == key =>
		{
			Some(node)
		}
		_ => None,
	}
}

pub fn overlay_summary_value<E>(node: &MenuNode<E>) -> String {
	match node {
		MenuNode::SectionSelect { groups, .. } => {
			selected_select_label(groups).unwrap_or("—").to_string()
		}
		MenuNode::BlockAsset { choices, .. } => choices
			.iter()
			.find(|choice| choice.selected)
			.map(|choice| choice.label.to_string())
			.unwrap_or_else(|| "—".into()),
		MenuNode::ItemMultiSelect { rows, .. } => {
			let worn = rows.iter().filter(|row| row.asset.selected).count();
			format!("{worn} worn")
		}
		_ => "—".into(),
	}
}

fn selected_select_label<E>(groups: &[SelectGroup<E>]) -> Option<&'static str> {
	groups
		.iter()
		.flat_map(|group| group.choices.iter())
		.find(|choice| choice.selected)
		.map(|choice| choice.label)
}

pub fn spawn_overlay_summary<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &'static str,
	node: &MenuNode<E>,
	justify: JustifyContent,
) {
	spawn_select_summary(
		parent,
		fonts,
		label,
		&overlay_summary_value(node),
		justify,
		crate::widgets::OpenSelectKey(label),
	);
}

/// Full-screen picker chrome. The host fills the returned viewport.
pub fn spawn_overlay_shell(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	title: &str,
) -> Entity {
	let mut viewport = Entity::PLACEHOLDER;
	parent
		.spawn((
			crate::widgets::OverlaySelectRoot,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|root| {
			root.spawn((
				Button,
				CloseOverlaySelect,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(0.0),
					top: Val::Px(0.0),
					width: Val::Percent(100.0),
					height: Val::Percent(100.0),
					..default()
				},
				BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.88)),
			));
			root.spawn((
				Node {
					position_type: PositionType::Absolute,
					left: Val::Percent(6.0),
					top: Val::Percent(8.0),
					width: Val::Percent(62.0),
					height: Val::Percent(84.0),
					flex_direction: FlexDirection::Column,
					row_gap: Val::Px(PANEL_ROW_GAP),
					padding: UiRect::all(Val::Px(8.0)),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|panel| {
				panel
					.spawn((
						Node {
							width: Val::Percent(100.0),
							flex_direction: FlexDirection::Row,
							justify_content: JustifyContent::SpaceBetween,
							align_items: AlignItems::Center,
							..default()
						},
						Pickable::IGNORE,
					))
					.with_children(|header| {
						spawn_block_label(header, fonts, title);
						spawn_text_button(header, fonts, "back", CloseOverlaySelect);
					});
				viewport = menu_components::spawn_scroll_pane(
					panel,
					crate::widgets::OverlaySelectViewport,
					AlignItems::FlexStart,
					PANEL_ROW_GAP,
				);
			});
		});
	viewport
}

pub fn render_overlay_body<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	node: &MenuNode<E>,
	parent: &mut ChildSpawnerCommands,
	context: &mut RenderContext<'_, C>,
	justify: MenuJustify,
) {
	match node {
		MenuNode::SectionSelect { groups, .. } => {
			for group in groups {
				if let Some(group_label) = group.label {
					spawn_group_label(parent, context.fonts, group_label);
				}
				spawn_tile_grid(parent, justify.content(), |grid| {
					for choice in &group.choices {
						spawn_asset_tile(
							grid,
							context.fonts,
							choice.label,
							choice.selected,
							None,
							MenuButton(choice.event),
						);
					}
				});
			}
		}
		MenuNode::BlockAsset { preview, choices, .. } => {
			let preview = bevy_color(*preview);
			spawn_tile_grid(parent, justify.content(), |grid| {
				for choice in choices {
					let thumbnail = asset_thumbnail(choice, preview, context);
					spawn_asset_tile(
						grid,
						context.fonts,
						choice.label,
						choice.selected,
						thumbnail,
						MenuButton(choice.event),
					);
				}
			});
		}
		MenuNode::ItemMultiSelect { rows, .. } => {
			for row in rows {
				overlay_item_row(row, parent, context, justify);
			}
		}
		_ => {}
	}
}

fn overlay_item_row<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	row: &ItemRow<E>,
	parent: &mut ChildSpawnerCommands,
	context: &mut RenderContext<'_, C>,
	justify: MenuJustify,
) {
	let thumbnail = asset_thumbnail(&row.asset, bevy_color(row.preview), context);
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(PANEL_ROW_GAP),
				row_gap: Val::Px(PANEL_ROW_GAP),
				align_items: AlignItems::Center,
				justify_content: justify.content(),
				flex_wrap: FlexWrap::Wrap,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|item| {
			spawn_asset_tile(
				item,
				context.fonts,
				row.asset.label,
				row.asset.selected,
				thumbnail,
				MenuButton(row.asset.event),
			);
			spawn_swatch_row(item, justify.content(), |swatches| {
				for choice in &row.colors {
					spawn_swatch(
						swatches,
						choice.color_hex,
						choice.selected,
						MenuButton(choice.event),
					);
				}
			});
		});
}

#[cfg(test)]
mod tests {
	use super::{find_overlay_node, overlay_closes_on_pick, overlay_select_label};
	use character_ui_menu::{MenuNode, SelectChoice, SelectGroup};

	#[test]
	fn long_catalogs_use_overlay() {
		assert!(overlay_select_label("Species"));
		assert!(overlay_select_label("Clothing"));
		assert!(overlay_select_label("Animation"));
		assert!(overlay_select_label("Clip"));
		assert!(!overlay_select_label("Eyes"));
		assert!(!overlay_select_label("Hair"));
	}

	#[test]
	fn clothing_stays_open() {
		assert!(!overlay_closes_on_pick("Clothing"));
		assert!(overlay_closes_on_pick("Species"));
	}

	#[test]
	fn finds_nested_block() {
		let tree = [MenuNode::<u8>::Section {
			label: "Animation",
			children: vec![MenuNode::BlockAsset {
				label: "Clip",
				preview: character_ui_menu::PreviewColor::WHITE,
				choices: vec![],
			}],
		}];
		assert!(matches!(
			find_overlay_node(&tree, "Clip"),
			Some(MenuNode::BlockAsset { label: "Clip", .. })
		));
	}

	#[test]
	fn finds_section_select() {
		let tree = [MenuNode::<u8>::SectionSelect {
			label: "Species",
			groups: vec![SelectGroup::unlabeled(vec![SelectChoice {
				label: "braidman",
				selected: true,
				event: 1,
			}])],
			children: vec![],
		}];
		assert!(matches!(
			find_overlay_node(&tree, "Species"),
			Some(MenuNode::SectionSelect { label: "Species", .. })
		));
	}
}
