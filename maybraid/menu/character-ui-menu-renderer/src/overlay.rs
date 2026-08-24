//! Overlay routing. The IR stays `MenuNode`; this crate decides what opens
//! a picker and what paints inside it.

use bevy::prelude::*;
use character_ui_menu::{ItemRow, MenuNode, SelectGroup};

use crate::sink::{
	asset_thumbnail, bevy_color, MaybraidMenuSink, MenuSink, MenuThumbnailContext, RenderContext,
};
use crate::widgets::{CloseOverlaySelect, MenuButton};
use crate::MenuJustify;
use menu_components::{
	spawn_asset_tile, spawn_group_label, spawn_panel_title, spawn_swatch, spawn_swatch_row,
	spawn_text_button, spawn_tile_grid, HudFonts, PANEL_ROW_GAP,
};

/// Catalog nodes that can show a selected name on a header.
pub fn is_select_node<E>(node: &MenuNode<E>) -> bool {
	matches!(
		node,
		MenuNode::SectionSelect { .. }
			| MenuNode::BlockAsset { .. }
			| MenuNode::ItemMultiSelect { .. }
	)
}

/// Flatten fragments so a section's real children can be inspected.
pub fn flatten_nodes<E>(nodes: &[MenuNode<E>]) -> Vec<&MenuNode<E>> {
	let mut out = Vec::new();
	flatten_into(nodes, &mut out);
	out
}

fn flatten_into<'a, E>(nodes: &'a [MenuNode<E>], out: &mut Vec<&'a MenuNode<E>>) {
	for node in nodes {
		match node {
			MenuNode::Fragment(children) => flatten_into(children, out),
			other => out.push(other),
		}
	}
}

/// The lone catalog under a section, if there is exactly one.
pub fn primary_select<'a, E>(children: &'a [MenuNode<E>]) -> Option<&'a MenuNode<E>> {
	let selects: Vec<_> = flatten_nodes(children)
		.into_iter()
		.filter(|node| is_select_node(node))
		.collect();
	(selects.len() == 1).then(|| selects[0])
}

/// True when the overlay is only a single catalog (no sliders/swatches).
pub fn is_picker_only<E>(node: &MenuNode<E>) -> bool {
	match node {
		MenuNode::SectionSelect { .. } | MenuNode::BlockAsset { .. } => true,
		MenuNode::ItemMultiSelect { .. } => false,
		MenuNode::Section { children, .. } => {
			let flat = flatten_nodes(children);
			flat.len() == 1 && is_select_node(flat[0]) && is_picker_only(flat[0])
		}
		_ => false,
	}
}

pub fn overlay_closes_on_pick<E>(node: &MenuNode<E>) -> bool {
	is_picker_only(node)
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
		MenuNode::Section { label, children } => {
			if *label == key {
				Some(node)
			} else {
				find_overlay_node(children, key)
			}
		}
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
		MenuNode::Section { children, .. } => {
			primary_select(children).map(overlay_summary_value).unwrap_or_default()
		}
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
		_ => String::new(),
	}
}

fn selected_select_label<E>(groups: &[SelectGroup<E>]) -> Option<&'static str> {
	groups
		.iter()
		.flat_map(|group| group.choices.iter())
		.find(|choice| choice.selected)
		.map(|choice| choice.label)
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
						spawn_panel_title(header, fonts, title);
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
		MenuNode::Section { label, children } => {
			if let Some(select) = picker_only_child(children) {
				render_overlay_body(select, parent, context, justify);
			} else {
				MaybraidMenuSink { justify, interior: true, omit_block_label: Some(label) }
					.render_nodes(children, parent, context);
			}
		}
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
		other => {
			MaybraidMenuSink { justify, interior: true, omit_block_label: None }
				.render_node(other, parent, context);
		}
	}
}

fn picker_only_child<'a, E>(children: &'a [MenuNode<E>]) -> Option<&'a MenuNode<E>> {
	let flat = flatten_nodes(children);
	(flat.len() == 1 && is_picker_only(flat[0])).then(|| flat[0])
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
	use super::{
		find_overlay_node, is_picker_only, overlay_closes_on_pick, overlay_summary_value,
		primary_select,
	};
	use character_ui_menu::{MenuNode, PreviewColor, SelectChoice, SelectGroup};

	fn clip() -> MenuNode<u8> {
		MenuNode::BlockAsset { label: "Clip", preview: PreviewColor::WHITE, choices: vec![] }
	}

	#[test]
	fn animation_section_is_a_picker() {
		let section = MenuNode::<u8>::Section { label: "Animation", children: vec![clip()] };
		assert!(is_picker_only(&section));
		assert!(overlay_closes_on_pick(&section));
	}

	#[test]
	fn mixed_section_stays_open() {
		let section = MenuNode::<u8>::Section {
			label: "Hair",
			children: vec![
				clip(),
				MenuNode::LabeledSwatch { label: "Hair Color", choices: vec![] },
			],
		};
		assert!(!is_picker_only(&section));
		assert!(!overlay_closes_on_pick(&section));
		assert!(primary_select(match &section {
			MenuNode::Section { children, .. } => children,
			_ => unreachable!(),
		})
		.is_some());
	}

	#[test]
	fn species_header_shows_selected_name() {
		let node = MenuNode::<u8>::SectionSelect {
			label: "Species",
			groups: vec![SelectGroup::unlabeled(vec![SelectChoice {
				label: "braidman",
				selected: true,
				event: 1,
			}])],
			children: vec![],
		};
		assert_eq!(overlay_summary_value(&node), "braidman");
	}

	#[test]
	fn empty_section_has_no_primary_select() {
		assert!(primary_select::<u8>(&[]).is_none());
		assert!(!is_picker_only(&MenuNode::<u8>::Section { label: "Head", children: vec![] }));
	}

	#[test]
	fn finds_section() {
		let tree = [MenuNode::<u8>::Section { label: "Head", children: vec![] }];
		assert!(matches!(
			find_overlay_node(&tree, "Head"),
			Some(MenuNode::Section { label: "Head", .. })
		));
	}
}
