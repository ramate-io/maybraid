//! Forward contract from the [`MenuNode`] IR to Maybraid HUD widgets.

use bevy::prelude::*;
use character_ui_menu::{
	AssetChoice, AssetThumbnailDisplay, GridCatalogChoice, ItemRow, MenuNode, PreviewColor,
	SwatchChoice, ThumbnailRequest,
};
use menu_components::{
	HudFonts, HudMenu, HudMenuItem, PANEL_LABEL_FONT_SIZE, PANEL_ROW_GAP, ShortTextField,
	ShortTextKey, TEXT_YELLOW, spawn_asset_tile, spawn_grid_catalog_tile, spawn_group_label,
	spawn_hud_text, spawn_labeled_row, spawn_section_header, spawn_short_text_button,
	spawn_stepper, spawn_swatch, spawn_swatch_row, spawn_tile_grid,
};

use crate::justify::MenuJustify;
use crate::overlay::{overlay_summary_value, primary_select};
use crate::widgets::{MenuButton, OpenSelectKey};

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
	pub thumbnails: &'a mut T,
	pub asset_thumbnails: AssetThumbnailDisplay,
	pub prewarm: &'a mut Vec<ThumbnailRequest>,
	pub hud_menu: Entity,
	pub hud_item_count: usize,
}

impl<T> RenderContext<'_, T> {
	pub fn stamp_hud_item(&mut self) -> HudMenuItem {
		let item = HudMenuItem { index: self.hud_item_count, menu: self.hud_menu };
		self.hud_item_count += 1;
		item
	}

	pub fn hud_menu(&self, previous: Option<HudMenu>) -> HudMenu {
		HudMenu::retain(self.hud_item_count, previous)
	}
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
	/// When true, catalogs paint inline (inside an overlay). The panel path
	/// only stamps headers that open a picker.
	pub interior: bool,
}

impl MaybraidMenuSink {
	pub fn new(justify: MenuJustify) -> Self {
		Self { justify, interior: false }
	}

	pub fn overlay(justify: MenuJustify) -> Self {
		Self { justify, interior: true }
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
				if self.interior {
					self.select_grid(groups, parent, context);
				} else {
					self.header(parent, context, label, Some(overlay_summary_value(node)));
					self.render_nodes(children, parent, context);
				}
			}
			MenuNode::LabeledCycle { label, value, minus, plus } => {
				self.labeled_control(parent, context, label, |row, context| {
					spawn_stepper(
						row,
						context.fonts,
						"<",
						">",
						value,
						(MenuButton(*minus), context.stamp_hud_item()),
						(MenuButton(*plus), context.stamp_hud_item()),
					);
				});
			}
			MenuNode::LabeledSlider { label, value, decrease, increase } => {
				self.labeled_control(parent, context, label, |row, context| {
					spawn_stepper(
						row,
						context.fonts,
						"−",
						"+",
						&format!("{value:.2}"),
						(MenuButton(*decrease), context.stamp_hud_item()),
						(MenuButton(*increase), context.stamp_hud_item()),
					);
				});
			}
			MenuNode::LabeledSwatch { label, choices } => {
				self.labeled_control(parent, context, label, |row, context| {
					self.swatch_row(row, context, choices);
				});
			}
			MenuNode::BlockAsset { label, preview, choices } => {
				if self.interior {
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
								(MenuButton(choice.event), context.stamp_hud_item()),
							);
						}
					});
				} else {
					self.header(parent, context, label, Some(overlay_summary_value(node)));
				}
			}
			MenuNode::ItemMultiSelect { label, rows } => {
				if self.interior {
					for row in rows {
						self.item_row(row, parent, context);
					}
				} else {
					self.header(parent, context, label, Some(overlay_summary_value(node)));
				}
			}
			MenuNode::GridCatalog { label, choices, .. } => {
				if self.interior {
					self.grid_catalog(choices, parent, context);
				} else {
					self.header(parent, context, label, Some(overlay_summary_value(node)));
				}
			}
			MenuNode::ShortText { label, value, max_len } => {
				self.short_text(parent, context, label, value, *max_len);
			}
		}
	}
}

impl MaybraidMenuSink {
	fn short_text<C>(
		&self,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
		label: &'static str,
		value: &str,
		max_len: usize,
	) {
		spawn_short_text_button(
			parent,
			context.fonts,
			label,
			value,
			false,
			self.justify.content(),
			(
				ShortTextKey(label),
				ShortTextField { value: value.to_string(), max_len, editing: false },
				context.stamp_hud_item(),
			),
		);
	}

	fn header<C>(
		&self,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
		label: &'static str,
		value: Option<String>,
	) {
		spawn_section_header(
			parent,
			context.fonts,
			label,
			value.as_deref(),
			self.justify.content(),
			(OpenSelectKey(label), context.stamp_hud_item()),
		);
	}

	fn labeled_control<C>(
		&self,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
		label: &str,
		controls: impl FnOnce(&mut ChildSpawnerCommands, &mut RenderContext<'_, C>),
	) {
		spawn_labeled_row(parent, self.justify.content(), |row| {
			spawn_hud_text(
				row,
				context.fonts.item(PANEL_LABEL_FONT_SIZE),
				label,
				TEXT_YELLOW,
				bevy::text::Justify::Left,
			);
			controls(row, context);
		});
	}

	fn section<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
		&self,
		label: &'static str,
		children: &[MenuNode<E>],
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		if self.interior {
			self.render_nodes(children, parent, context);
			return;
		}
		let value = primary_select(children).map(overlay_summary_value);
		self.header(parent, context, label, value);
	}

	fn select_grid<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
		&self,
		groups: &[character_ui_menu::SelectGroup<E>],
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		for group in groups {
			if let Some(group_label) = group.label {
				spawn_group_label(parent, context.fonts, group_label);
			}
			spawn_tile_grid(parent, self.justify.content(), |grid| {
				for choice in &group.choices {
					spawn_asset_tile(
						grid,
						context.fonts,
						choice.label,
						choice.selected,
						None,
						(MenuButton(choice.event), context.stamp_hud_item()),
					);
				}
			});
		}
	}

	fn grid_catalog<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
		&self,
		choices: &[GridCatalogChoice<E>],
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		spawn_tile_grid(parent, self.justify.content(), |grid| {
			for choice in choices {
				let thumbnail = grid_catalog_thumbnail(choice, bevy_color(choice.preview), context);
				spawn_grid_catalog_tile(
					grid,
					context.fonts,
					choice.label,
					choice.selected,
					thumbnail,
					(MenuButton(choice.event), context.stamp_hud_item()),
				);
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
					(MenuButton(row.asset.event), context.stamp_hud_item()),
				);
				self.swatch_row(item, context, &row.colors);
			});
	}

	fn swatch_row<E: Copy + Send + Sync + 'static, C>(
		&self,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
		choices: &[SwatchChoice<E>],
	) {
		spawn_swatch_row(parent, self.justify.content(), |row| {
			for choice in choices {
				spawn_swatch(
					row,
					choice.color_hex,
					choice.selected,
					(MenuButton(choice.event), context.stamp_hud_item()),
				);
			}
		});
	}
}

pub(crate) fn asset_thumbnail<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	choice: &AssetChoice<E>,
	preview: Color,
	context: &mut RenderContext<'_, C>,
) -> Option<Handle<Image>> {
	thumbnail_image(choice.label, choice.path, choice.thumbnail_camera, preview, context)
}

fn grid_catalog_thumbnail<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	choice: &GridCatalogChoice<E>,
	preview: Color,
	context: &mut RenderContext<'_, C>,
) -> Option<Handle<Image>> {
	thumbnail_image(choice.label, choice.path, choice.thumbnail_camera, preview, context)
}

fn thumbnail_image<C: MenuThumbnailContext>(
	label: &'static str,
	path: &'static str,
	camera: character_ui_menu::ThumbnailCamera,
	preview: Color,
	context: &mut RenderContext<'_, C>,
) -> Option<Handle<Image>> {
	if context.asset_thumbnails != AssetThumbnailDisplay::None && !path.is_empty() {
		context.prewarm.push(ThumbnailRequest::new(path, color_key(preview), camera));
	}
	match context.asset_thumbnails {
		AssetThumbnailDisplay::Inline => {
			context.thumbnails.image_for_asset(label, path, preview, camera)
		}
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
