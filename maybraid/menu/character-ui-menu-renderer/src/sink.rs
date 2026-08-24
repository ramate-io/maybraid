//! Forward contract from the [`MenuNode`] IR to Maybraid HUD widgets.

use bevy::prelude::*;
use character_ui_menu::{
	AssetChoice, AssetThumbnailDisplay, ItemRow, MenuNode, PreviewColor, SectionOpen, SwatchChoice,
	ThumbnailRequest,
};
use menu_components::{
	spawn_asset_tile, spawn_block_label, spawn_group_label, spawn_hud_text, spawn_labeled_row,
	spawn_section_header, spawn_select_row, spawn_stepper, spawn_swatch, spawn_swatch_row,
	spawn_tile_grid, HudFonts, PANEL_LABEL_FONT_SIZE, PANEL_ROW_GAP, TEXT_YELLOW,
};

use crate::justify::MenuJustify;
use crate::overlay::{overlay_select_label, spawn_overlay_summary};
use crate::widgets::{MenuButton, ToggleSectionKey};

/// Renderer-owned thumbnail bridge. The host adapts this to its cache.
pub trait MenuThumbnailContext {
	fn image_for_asset(
		&mut self,
		label: &'static str,
		asset_path: &'static str,
		color: Color,
		camera: character_ui_menu::ThumbnailCamera,
	) -> Option<Handle<Image>>;
}

/// Thumbnail context that never returns an image.
#[derive(Default)]
pub struct NoThumbnails;

impl MenuThumbnailContext for NoThumbnails {
	fn image_for_asset(
		&mut self,
		_label: &'static str,
		_asset_path: &'static str,
		_color: Color,
		_camera: character_ui_menu::ThumbnailCamera,
	) -> Option<Handle<Image>> {
		None
	}
}

/// Per-rebuild rendering state shared across the whole node tree.
pub struct RenderContext<'a, T> {
	pub fonts: &'a HudFonts,
	pub sections: &'a dyn SectionOpen,
	pub thumbnails: &'a mut T,
	pub asset_thumbnails: AssetThumbnailDisplay,
	pub prewarm: &'a mut Vec<ThumbnailRequest>,
}

/// Forward dispatcher over [`MenuNode`] trees.
pub trait MenuSink<E: Copy + Send + Sync + 'static> {
	fn render_node<C: MenuThumbnailContext>(
		&self,
		node: &MenuNode<E>,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	);

	fn render_nodes<C: MenuThumbnailContext>(
		&self,
		nodes: &[MenuNode<E>],
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		for node in nodes {
			self.render_node(node, parent, context);
		}
	}
}

/// Maybraid HUD implementation of the menu forward contract.
#[derive(Clone, Copy, Debug, Default)]
pub struct MaybraidMenuSink {
	pub justify: MenuJustify,
}

impl MaybraidMenuSink {
	pub fn new(justify: MenuJustify) -> Self {
		Self { justify }
	}
}

impl<E: Copy + Send + Sync + 'static> MenuSink<E> for MaybraidMenuSink {
	fn render_node<C: MenuThumbnailContext>(
		&self,
		node: &MenuNode<E>,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		match node {
			MenuNode::Fragment(children) => self.render_nodes(children, parent, context),
			MenuNode::Section { label, children } => self.section(label, children, parent, context),
			MenuNode::SectionSelect { label, groups, children } => {
				if overlay_select_label(label) {
					spawn_overlay_summary(
						parent,
						context.fonts,
						label,
						node,
						self.justify.content(),
					);
				} else {
					spawn_block_label(parent, context.fonts, label);
					for group in groups {
						if let Some(group_label) = group.label {
							spawn_group_label(parent, context.fonts, group_label);
						}
						parent
							.spawn((
								Node {
									width: Val::Percent(100.0),
									flex_direction: FlexDirection::Column,
									align_items: self.justify.items(),
									row_gap: Val::Px(4.0),
									..default()
								},
								Pickable::IGNORE,
							))
							.with_children(|list| {
								for choice in &group.choices {
									spawn_select_row(
										list,
										context.fonts,
										choice.label,
										choice.selected,
										self.justify.content(),
										MenuButton(choice.event),
									);
								}
							});
					}
				}
				self.render_nodes(children, parent, context);
			}
			MenuNode::LabeledCycle { label, value, minus, plus } => {
				self.labeled_control(parent, context.fonts, label, |row| {
					spawn_stepper(
						row,
						context.fonts,
						"<",
						">",
						value,
						MenuButton(*minus),
						MenuButton(*plus),
					);
				});
			}
			MenuNode::LabeledSlider { label, value, decrease, increase } => {
				self.labeled_control(parent, context.fonts, label, |row| {
					spawn_stepper(
						row,
						context.fonts,
						"−",
						"+",
						&format!("{value:.2}"),
						MenuButton(*decrease),
						MenuButton(*increase),
					);
				});
			}
			MenuNode::LabeledSwatch { label, choices } => {
				self.labeled_control(parent, context.fonts, label, |row| {
					self.swatch_row(row, choices);
				});
			}
			MenuNode::BlockAsset { label, preview, choices } => {
				if overlay_select_label(label) {
					spawn_overlay_summary(
						parent,
						context.fonts,
						label,
						node,
						self.justify.content(),
					);
				} else {
					spawn_block_label(parent, context.fonts, label);
					let preview = bevy_color(*preview);
					spawn_tile_grid(parent, self.justify.content(), |grid| {
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
			}
			MenuNode::ItemMultiSelect { label, rows } => {
				if overlay_select_label(label) {
					spawn_overlay_summary(
						parent,
						context.fonts,
						label,
						node,
						self.justify.content(),
					);
				} else {
					spawn_block_label(parent, context.fonts, label);
					for row in rows {
						self.item_row(row, parent, context);
					}
				}
			}
		}
	}
}

impl MaybraidMenuSink {
	fn labeled_control(
		&self,
		parent: &mut ChildSpawnerCommands,
		fonts: &HudFonts,
		label: &str,
		controls: impl FnOnce(&mut ChildSpawnerCommands),
	) {
		spawn_labeled_row(parent, self.justify.content(), |row| {
			spawn_hud_text(
				row,
				fonts.item(PANEL_LABEL_FONT_SIZE),
				label,
				TEXT_YELLOW,
				bevy::text::Justify::Left,
			);
			controls(row);
		});
	}

	fn section<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
		&self,
		label: &'static str,
		children: &[MenuNode<E>],
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		let open = context.sections.is_open(label);
		parent
			.spawn((
				Node {
					width: Val::Percent(100.0),
					flex_direction: FlexDirection::Column,
					align_items: self.justify.items(),
					row_gap: Val::Px(PANEL_ROW_GAP),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|section| {
				spawn_section_header(
					section,
					context.fonts,
					label,
					open,
					self.justify.content(),
					ToggleSectionKey(label),
				);
				if open {
					self.render_nodes(children, section, context);
				}
			});
	}

	fn item_row<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
		&self,
		row: &ItemRow<E>,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
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
					justify_content: self.justify.content(),
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
				self.swatch_row(item, &row.colors);
			});
	}

	fn swatch_row<E: Copy + Send + Sync + 'static>(
		&self,
		parent: &mut ChildSpawnerCommands,
		choices: &[SwatchChoice<E>],
	) {
		spawn_swatch_row(parent, self.justify.content(), |row| {
			for choice in choices {
				spawn_swatch(row, choice.color_hex, choice.selected, MenuButton(choice.event));
			}
		});
	}
}

pub(crate) fn asset_thumbnail<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	choice: &AssetChoice<E>,
	preview: Color,
	context: &mut RenderContext<'_, C>,
) -> Option<Handle<Image>> {
	if context.asset_thumbnails != AssetThumbnailDisplay::None && !choice.path.is_empty() {
		context.prewarm.push(ThumbnailRequest::new(
			choice.path,
			color_key(preview),
			choice.thumbnail_camera,
		));
	}
	match context.asset_thumbnails {
		AssetThumbnailDisplay::Inline => context.thumbnails.image_for_asset(
			choice.label,
			choice.path,
			preview,
			choice.thumbnail_camera,
		),
		_ => None,
	}
}

pub(crate) fn bevy_color(preview: PreviewColor) -> Color {
	Color::srgba(preview.red, preview.green, preview.blue, preview.alpha)
}

fn color_key(color: Color) -> [u8; 4] {
	let srgba = color.to_srgba();
	[
		(srgba.red * 255.0) as u8,
		(srgba.green * 255.0) as u8,
		(srgba.blue * 255.0) as u8,
		(srgba.alpha * 255.0) as u8,
	]
}
