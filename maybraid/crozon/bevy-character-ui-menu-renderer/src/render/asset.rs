use bevy::prelude::*;
use character_ui_menu::{
	AssetOption, AssetSingleSelect, AssetThumbnailDisplay, LabelOption, ListValues,
};

use crate::render::util::color_to_key;
use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{render_asset_button, text};

pub trait AssetEventMap<E: Copy + Send + Sync + 'static, T: Copy> {
	fn select_event(&self, value: T) -> E;
}

/// Asset grid wired to menu events.
pub struct AssetSelect<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: AssetEventMap<E, T> + Copy,
{
	pub select: AssetSingleSelect<T>,
	pub map: M,
	_marker: core::marker::PhantomData<E>,
}

impl<E, T, M> AssetSelect<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: AssetEventMap<E, T> + Copy,
{
	pub const fn new(select: AssetSingleSelect<T>, map: M) -> Self {
		Self { select, map, _marker: core::marker::PhantomData }
	}
}

impl<E, T, M> RenderMenu for AssetSelect<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
	M: AssetEventMap<E, T> + Copy,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		AssetGrid::new(self.select.value, self.map).render_with(renderer, parent, context);
	}
}

/// Grid of asset picker buttons.
pub struct AssetGrid<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: AssetEventMap<E, T> + Copy,
{
	pub active: T,
	pub map: M,
	_marker: core::marker::PhantomData<E>,
}

impl<E, T, M> AssetGrid<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: AssetEventMap<E, T> + Copy,
{
	pub const fn new(active: T, map: M) -> Self {
		Self { active, map, _marker: core::marker::PhantomData }
	}
}

impl<E, T, M> RenderMenu for AssetGrid<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
	M: AssetEventMap<E, T> + Copy,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		let preview_color = context.preview_color;
		parent
			.spawn((
				Node {
					width: Val::Percent(100.0),
					flex_direction: FlexDirection::Row,
					flex_wrap: FlexWrap::Wrap,
					column_gap: Val::Px(8.0),
					row_gap: Val::Px(crate::widgets::MENU_VERTICAL_GAP),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|grid| {
				for value in T::values() {
					let value = *value;
					let asset = value.asset();
					if context.asset_thumbnails != AssetThumbnailDisplay::None && !asset.path.is_empty()
					{
						context.prewarm.push(character_ui_menu::ThumbnailRequest::new(
							asset.path,
							color_to_key(preview_color),
							asset.thumbnail_camera,
						));
					}
					let thumbnail = match context.asset_thumbnails {
						AssetThumbnailDisplay::Inline => context.thumbnails.image_for_asset(
							asset.label,
							asset.path,
							preview_color,
							asset.thumbnail_camera,
						),
						_ => None,
					};
					render_asset_button(
						grid,
						value.label(),
						self.map.select_event(value),
						value == self.active,
						thumbnail,
					);
				}
			});
	}
}

impl<T> RenderMenu for AssetSingleSelect<T>
where
	T: Copy + LabelOption,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
		text(parent, self.value.label(), 11.0, Color::srgb(0.85, 0.95, 1.0));
	}
}
