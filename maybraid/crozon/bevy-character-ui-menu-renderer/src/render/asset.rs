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

/// Section heading plus a wrapped grid of asset pickers.
pub struct LabeledAssetGrid<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: AssetEventMap<E, T> + ?Sized,
{
	pub label: &'static str,
	pub active: T,
	pub map: &'a M,
	_marker: core::marker::PhantomData<E>,
}

impl<'a, E, T, M> LabeledAssetGrid<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: AssetEventMap<E, T> + ?Sized,
{
	pub const fn new(label: &'static str, active: T, map: &'a M) -> Self {
		Self { label, active, map, _marker: core::marker::PhantomData }
	}
}

impl<'a, E, T, M> RenderMenu for LabeledAssetGrid<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
	M: AssetEventMap<E, T> + ?Sized,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		text(parent, self.label, 12.0, Color::srgb(0.78, 0.84, 0.92));
		AssetGrid::new(self.active, self.map).render_with(renderer, parent, context);
	}
}

/// Grid of asset picker buttons.
pub struct AssetGrid<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: AssetEventMap<E, T> + ?Sized,
{
	pub active: T,
	pub map: &'a M,
	_marker: core::marker::PhantomData<E>,
}

impl<'a, E, T, M> AssetGrid<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: AssetEventMap<E, T> + ?Sized,
{
	pub const fn new(active: T, map: &'a M) -> Self {
		Self { active, map, _marker: core::marker::PhantomData }
	}
}

impl<'a, E, T, M> RenderMenu for AssetGrid<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
	M: AssetEventMap<E, T> + ?Sized,
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
					column_gap: Val::Px(6.0),
					row_gap: Val::Px(6.0),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|grid| {
				for value in T::values() {
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
						self.map.select_event(*value),
						*value == self.active,
						thumbnail,
					);
				}
			});
	}
}

impl<T> RenderMenu for AssetSingleSelect<T>
where
	T: Copy + LabelOption + ListValues + AssetOption,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		_parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
	}
}
