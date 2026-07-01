use bevy::prelude::*;
use character_ui_menu::{
	AssetOption, AssetSingleSelect, AssetThumbnailDisplay, LabelOption, ListValues, MultiSelect,
	Root, Section, SectionOpen, SingleSelect, Slider, SwatchOption, SwatchSingleSelect,
	ThumbnailRequest,
};

use crate::widgets::{
	color_from_hex, render_asset_button, render_button, row_node, text, ToggleSectionKey, ACTIVE,
	INACTIVE, MUTED,
};

/// Renderer-owned thumbnail bridge. The playground adapts this to its cache.
pub trait MenuThumbnailContext {
	fn image_for_asset(
		&mut self,
		label: &'static str,
		asset_path: &'static str,
		color: Color,
		camera: character_ui_menu::ThumbnailCamera,
	) -> Option<Handle<Image>>;
}

pub struct RenderContext<'a, T> {
	pub sections: &'a dyn SectionOpen,
	pub thumbnails: &'a mut T,
	pub asset_thumbnails: AssetThumbnailDisplay,
	pub preview_color: Color,
	pub base_preview_color: Color,
	pub accent_preview_color: Color,
	pub prewarm: &'a mut Vec<ThumbnailRequest>,
}

#[derive(Default)]
pub struct Renderer;

pub trait RenderMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	);
}

impl Renderer {
	pub fn render<M, C>(
		&self,
		parent: &mut ChildSpawnerCommands,
		menu: &M,
		context: &mut RenderContext<'_, C>,
	) where
		M: RenderMenu,
		C: MenuThumbnailContext,
	{
		menu.render_with(self, parent, context);
	}

	pub fn render_collapsible<T, C>(
		&self,
		parent: &mut ChildSpawnerCommands,
		label: &'static str,
		open: bool,
		body: &T,
		context: &mut RenderContext<'_, C>,
	) where
		T: RenderMenu,
		C: MenuThumbnailContext,
	{
		parent
			.spawn((
				Node {
					width: Val::Percent(100.0),
					flex_direction: FlexDirection::Column,
					row_gap: Val::Px(4.0),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|section_parent| {
				section_parent.spawn((
					Button,
					Node {
						min_width: Val::Px(28.0),
						height: Val::Px(crate::widgets::BUTTON_HEIGHT),
						padding: UiRect::axes(Val::Px(7.0), Val::Px(2.0)),
						justify_content: JustifyContent::Center,
						align_items: AlignItems::Center,
						..default()
					},
					BackgroundColor(if open { ACTIVE } else { INACTIVE }),
					ToggleSectionKey(label),
				))
				.with_children(|button| {
					text(
						button,
						&format!("{} {}", if open { "v" } else { ">" }, label),
						10.0,
						Color::WHITE,
					);
				});
				if open {
					body.render_with(self, section_parent, context);
				}
			});
	}

	pub fn render_cycle_row<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
		&self,
		parent: &mut ChildSpawnerCommands,
		label: &'static str,
		value_label: &'static str,
		cycle_minus: E,
		cycle_plus: E,
		_context: &mut RenderContext<'_, C>,
	) {
		parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
			text(row, label, 11.0, Color::WHITE);
			render_button(row, "<", cycle_minus, false);
			text(row, value_label, 11.0, Color::srgb(0.85, 0.95, 1.0));
			render_button(row, ">", cycle_plus, false);
		});
	}

	pub fn render_slider_row<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
		&self,
		parent: &mut ChildSpawnerCommands,
		label: &'static str,
		value: f32,
		step: f32,
		decrease: E,
		increase: E,
		_context: &mut RenderContext<'_, C>,
	) {
		parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
			text(row, label, 11.0, Color::WHITE);
			render_button(row, "-", decrease, false);
			text(row, &format!("{value:.2}"), 11.0, Color::srgb(0.85, 0.95, 1.0));
			render_button(row, "+", increase, false);
		});
	}

	pub fn render_swatch_row<E: Copy + Send + Sync + 'static, T, C: MenuThumbnailContext>(
		&self,
		parent: &mut ChildSpawnerCommands,
		label: &'static str,
		active: T,
		to_event: impl Fn(T) -> E,
		_context: &mut RenderContext<'_, C>,
	) where
		T: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
	{
		parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
			text(row, label, 11.0, Color::WHITE);
			for value in T::values() {
				let selected = *value == active;
				row.spawn((
					Button,
					Node {
						width: Val::Px(22.0),
						height: Val::Px(18.0),
						border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
						..default()
					},
					BorderColor::all(if selected { Color::WHITE } else { MUTED }),
					BackgroundColor(color_from_hex(value.color_hex())),
					crate::widgets::MenuButton(to_event(*value)),
				));
			}
		});
	}

	pub fn render_asset_grid<E: Copy + Send + Sync + 'static, T, C: MenuThumbnailContext>(
		&self,
		parent: &mut ChildSpawnerCommands,
		label: &'static str,
		active: T,
		to_event: impl Fn(T) -> E,
		preview_color: impl Fn(T) -> Color,
		context: &mut RenderContext<'_, C>,
	) where
		T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
	{
		text(parent, label, 12.0, Color::srgb(0.78, 0.84, 0.92));
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
					let color = preview_color(*value);
					if context.asset_thumbnails != AssetThumbnailDisplay::None && !asset.path.is_empty()
					{
						context.prewarm.push(ThumbnailRequest::new(
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
					render_asset_button(
						grid,
						value.label(),
						to_event(*value),
						*value == active,
						thumbnail,
					);
				}
			});
	}

	pub(crate) fn render_asset_list<E: Copy + Send + Sync + 'static, T, C: MenuThumbnailContext>(
		&self,
		parent: &mut ChildSpawnerCommands,
		label: &'static str,
		selected: &[T],
		to_event: impl Fn(T) -> E + Copy,
		preview_color: impl Fn(T) -> Color,
		context: &mut RenderContext<'_, C>,
	) where
		T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
	{
		text(parent, label, 12.0, Color::srgb(0.78, 0.84, 0.92));
		for value in T::values() {
			let asset = value.asset();
			let color = preview_color(*value);
			if context.asset_thumbnails != AssetThumbnailDisplay::None && !asset.path.is_empty() {
				context.prewarm.push(ThumbnailRequest::new(
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
			let active = selected.iter().any(|item| *item == *value);
			render_asset_button(parent, value.label(), to_event(*value), active, thumbnail);
		}
	}

	pub fn render_colored_multi_select<
		E: Copy + Send + Sync + 'static,
		T,
		C,
		Ctx: MenuThumbnailContext,
	>(
		&self,
		parent: &mut ChildSpawnerCommands,
		label: &'static str,
		selected: &[T],
		active_color: impl Fn(T) -> C,
		to_toggle_event: impl Fn(T) -> E + Copy,
		to_color_event: impl Fn(T, C) -> E + Copy,
		preview_color: impl Fn(T) -> Color + Copy,
		context: &mut RenderContext<'_, Ctx>,
	) where
		T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
		C: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
	{
		crate::widgets::text(parent, label, 12.0, Color::srgb(0.78, 0.84, 0.92));
		for value in T::values() {
			let asset = value.asset();
			let color = preview_color(*value);
			if context.asset_thumbnails != AssetThumbnailDisplay::None && !asset.path.is_empty() {
				context.prewarm.push(ThumbnailRequest::new(
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
			let active = selected.iter().any(|selected| *selected == *value);
			parent
				.spawn((
					Node {
						width: Val::Percent(100.0),
						flex_direction: FlexDirection::Row,
						column_gap: Val::Px(6.0),
						row_gap: Val::Px(4.0),
						align_items: AlignItems::Center,
						..default()
					},
					Pickable::IGNORE,
				))
				.with_children(|row| {
					render_asset_button(
						row,
						value.label(),
						to_toggle_event(*value),
						active,
						thumbnail,
					);
					self.render_inline_swatches(row, active_color(*value), |swatch| {
						to_color_event(*value, swatch)
					});
				});
		}
	}

	fn render_inline_swatches<E: Copy + Send + Sync + 'static, T>(
		&self,
		parent: &mut ChildSpawnerCommands,
		active: T,
		to_event: impl Fn(T) -> E,
	) where
		T: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
	{
		parent
			.spawn((
				Node {
					flex_direction: FlexDirection::Row,
					flex_wrap: FlexWrap::Wrap,
					column_gap: Val::Px(3.0),
					row_gap: Val::Px(3.0),
					align_items: AlignItems::Center,
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|row| {
				for value in T::values() {
					let selected = *value == active;
					row.spawn((
						Button,
						Node {
							width: Val::Px(20.0),
							height: Val::Px(16.0),
							border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
							..default()
						},
						BorderColor::all(if selected { Color::WHITE } else { MUTED }),
						BackgroundColor(color_from_hex(value.color_hex())),
						crate::widgets::MenuButton(to_event(*value)),
					));
				}
			});
	}
}

impl<T: RenderMenu> RenderMenu for Root<T> {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		self.value.render_with(renderer, parent, context);
	}
}

impl<T: RenderMenu> RenderMenu for Section<T> {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		let open = context.sections.is_open(self.label);
		renderer.render_collapsible(parent, self.label, open, &self.value, context);
	}
}

impl<T> RenderMenu for SingleSelect<T>
where
	T: Copy + LabelOption + ListValues,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		let _ = (renderer, parent, context);
	}
}

impl RenderMenu for Slider {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		_parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
	}
}

impl<T> RenderMenu for SwatchSingleSelect<T>
where
	T: Copy + LabelOption + ListValues + SwatchOption,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		_parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
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

impl<T> RenderMenu for MultiSelect<T> {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		_parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
	}
}

fn color_to_key(color: Color) -> [u8; 4] {
	[
		(color.to_srgba().red * 255.0) as u8,
		(color.to_srgba().green * 255.0) as u8,
		(color.to_srgba().blue * 255.0) as u8,
		(color.to_srgba().alpha * 255.0) as u8,
	]
}
