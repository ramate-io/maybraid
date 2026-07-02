use bevy::prelude::*;
use character_ui_menu::{
	AssetOption, AssetThumbnailDisplay, LabelOption, ListValues, MultiSelect, SwatchOption,
};

use crate::render::util::color_to_key;
use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{color_from_hex, inline_chip_row, render_asset_button, swatch_node, text, MUTED};

pub trait ToggleEventMap<E: Copy + Send + Sync + 'static, T: Copy> {
	fn toggle_event(&self, value: T) -> E;
}

pub trait ItemColorMap<T: Copy, C: Copy> {
	fn color_for(&self, item: T) -> C;
}

pub trait ItemPreviewColorMap<T: Copy> {
	fn preview_color(&self, item: T) -> Color;
}

pub trait ClothingSwatchEventMap<E: Copy + Send + Sync + 'static, T: Copy, C: Copy> {
	fn color_event(&self, item: T, color: C) -> E;
}

/// Asset multi-select with per-item inline color swatches.
pub struct ColoredAssetMultiSelect<'a, E, T, C, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	C: Copy,
	M: ColoredMultiSelectMaps<E, T, C> + ?Sized,
{
	pub label: &'static str,
	pub selected: Vec<T>,
	pub map: &'a M,
	_marker: core::marker::PhantomData<(E, C)>,
}

pub trait ColoredMultiSelectMaps<E: Copy + Send + Sync + 'static, T: Copy, C: Copy>:
	ToggleEventMap<E, T> + ClothingSwatchEventMap<E, T, C> + ItemColorMap<T, C> + ItemPreviewColorMap<T>
{
}

impl<'a, E, T, C, M> ColoredAssetMultiSelect<'a, E, T, C, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	C: Copy,
	M: ColoredMultiSelectMaps<E, T, C> + ?Sized,
{
	pub fn new(label: &'static str, selected: &[T], map: &'a M) -> Self {
		Self {
			label,
			selected: selected.to_vec(),
			map,
			_marker: core::marker::PhantomData,
		}
	}
}

impl<'a, E, T, C, M> RenderMenu for ColoredAssetMultiSelect<'a, E, T, C, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
	C: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
	M: ColoredMultiSelectMaps<E, T, C> + ?Sized,
{
	fn render_with<Ctx: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, Ctx>,
	) {
		character_ui_menu::BlockLabeled::new(
			self.label,
			ColoredAssetRows {
				selected: &self.selected,
				map: self.map,
				_marker: core::marker::PhantomData,
			},
		)
		.render_with(renderer, parent, context);
	}
}

struct ColoredAssetRows<'a, E, T, C, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	C: Copy,
	M: ColoredMultiSelectMaps<E, T, C> + ?Sized,
{
	selected: &'a [T],
	map: &'a M,
	_marker: core::marker::PhantomData<(E, C)>,
}

impl<'a, E, T, C, M> RenderMenu for ColoredAssetRows<'a, E, T, C, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
	C: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
	M: ColoredMultiSelectMaps<E, T, C> + ?Sized,
{
	fn render_with<Ctx: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, Ctx>,
	) {
		for value in T::values() {
			ColoredAssetRow::new(*value, self.selected, self.map).render_with(renderer, parent, context);
		}
	}
}

struct ColoredAssetRow<'a, E, T, C, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	C: Copy,
	M: ColoredMultiSelectMaps<E, T, C> + ?Sized,
{
	value: T,
	selected: &'a [T],
	map: &'a M,
	_marker: core::marker::PhantomData<(E, C)>,
}

impl<'a, E, T, C, M> ColoredAssetRow<'a, E, T, C, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	C: Copy,
	M: ColoredMultiSelectMaps<E, T, C> + ?Sized,
{
	const fn new(value: T, selected: &'a [T], map: &'a M) -> Self {
		Self { value, selected, map, _marker: core::marker::PhantomData }
	}
}

impl<'a, E, T, C, M> RenderMenu for ColoredAssetRow<'a, E, T, C, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
	C: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
	M: ColoredMultiSelectMaps<E, T, C> + ?Sized,
{
	fn render_with<Ctx: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, Ctx>,
	) {
		let asset = self.value.asset();
		let color = self.map.preview_color(self.value);
		if context.asset_thumbnails != AssetThumbnailDisplay::None && !asset.path.is_empty() {
			context.prewarm.push(character_ui_menu::ThumbnailRequest::new(
				asset.path,
				color_to_key(color),
				asset.thumbnail_camera,
			));
		}
		let thumbnail = match context.asset_thumbnails {
			AssetThumbnailDisplay::Inline => context.thumbnails.image_for_asset(
				asset.label,
				asset.path,
				color,
				asset.thumbnail_camera,
			),
			_ => None,
		};
		let active = self.selected.iter().any(|item| *item == self.value);
		parent
			.spawn((
				Node {
					width: Val::Percent(100.0),
					flex_direction: FlexDirection::Row,
					column_gap: Val::Px(6.0),
					row_gap: Val::Px(crate::widgets::MENU_VERTICAL_GAP),
					align_items: AlignItems::Center,
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|row| {
				render_asset_button(
					row,
					self.value.label(),
					self.map.toggle_event(self.value),
					active,
					thumbnail,
				);
				ClothingSwatchRow::new(self.value, self.map).render_with(renderer, row, context);
			});
	}
}

struct ClothingSwatchRow<'a, E, T, C, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	C: Copy,
	M: ColoredMultiSelectMaps<E, T, C> + ?Sized,
{
	item: T,
	map: &'a M,
	_marker: core::marker::PhantomData<(E, C)>,
}

impl<'a, E, T, C, M> ClothingSwatchRow<'a, E, T, C, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	C: Copy,
	M: ColoredMultiSelectMaps<E, T, C> + ?Sized,
{
	const fn new(item: T, map: &'a M) -> Self {
		Self { item, map, _marker: core::marker::PhantomData }
	}
}

impl<'a, E, T, C, M> RenderMenu for ClothingSwatchRow<'a, E, T, C, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	C: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
	M: ColoredMultiSelectMaps<E, T, C> + ?Sized,
{
	fn render_with<Ctx: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, Ctx>,
	) {
		let active = self.map.color_for(self.item);
		parent.spawn((inline_chip_row(), Pickable::IGNORE)).with_children(|row| {
			for swatch in C::values() {
				let swatch = *swatch;
				let selected = swatch == active;
				row.spawn((
					Button,
					swatch_node(selected),
					BorderColor::all(if selected { Color::WHITE } else { MUTED }),
					BackgroundColor(color_from_hex(swatch.color_hex())),
					crate::widgets::MenuButton(self.map.color_event(self.item, swatch)),
				));
			}
		});
	}
}

impl<T> RenderMenu for MultiSelect<T>
where
	T: Copy + LabelOption + ListValues + AssetOption,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
		let selected = self
			.selected
			.iter()
			.map(|item| item.label())
			.collect::<Vec<_>>()
			.join(", ");
		let summary = if selected.is_empty() { "none" } else { selected.as_str() };
		text(parent, summary, 11.0, Color::srgb(0.85, 0.95, 1.0));
	}
}
