use bevy::prelude::*;
use character_ui_menu::{LabelOption, ListValues, SwatchOption, SwatchSingleSelect};

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{color_from_hex, row_node, text, MUTED};

pub trait SwatchEventMap<E: Copy + Send + Sync + 'static, T: Copy> {
	fn swatch_event(&self, value: T) -> E;
}

/// Labeled row of color swatch buttons.
pub struct LabeledSwatch<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SwatchEventMap<E, T> + ?Sized,
{
	pub label: &'static str,
	pub active: T,
	pub map: &'a M,
	_marker: core::marker::PhantomData<E>,
}

impl<'a, E, T, M> LabeledSwatch<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SwatchEventMap<E, T> + ?Sized,
{
	pub const fn new(label: &'static str, active: T, map: &'a M) -> Self {
		Self { label, active, map, _marker: core::marker::PhantomData }
	}
}

impl<'a, E, T, M> RenderMenu for LabeledSwatch<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
	M: SwatchEventMap<E, T> + ?Sized,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
			text(row, self.label, 11.0, Color::WHITE);
			SwatchPicker::new(self.active, self.map).render_with(renderer, row, context);
		});
	}
}

/// Inline swatch buttons without a field label.
pub struct SwatchPicker<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SwatchEventMap<E, T> + ?Sized,
{
	pub active: T,
	pub map: &'a M,
	_marker: core::marker::PhantomData<E>,
}

impl<'a, E, T, M> SwatchPicker<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SwatchEventMap<E, T> + ?Sized,
{
	pub const fn new(active: T, map: &'a M) -> Self {
		Self { active, map, _marker: core::marker::PhantomData }
	}
}

impl<'a, E, T, M> RenderMenu for SwatchPicker<'a, E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
	M: SwatchEventMap<E, T> + ?Sized,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
		for value in T::values() {
			let value = *value;
			let selected = value == self.active;
			parent.spawn((
				Button,
				Node {
					width: Val::Px(22.0),
					height: Val::Px(18.0),
					border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
					..default()
				},
				BorderColor::all(if selected { Color::WHITE } else { MUTED }),
				BackgroundColor(color_from_hex(value.color_hex())),
				crate::widgets::MenuButton(self.map.swatch_event(value)),
			));
		}
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
