use bevy::prelude::*;
use character_ui_menu::{LabelOption, ListValues, SwatchOption, SwatchSingleSelect};

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{color_from_hex, inline_chip_row, swatch_node, MUTED};

pub trait SwatchEventMap<E: Copy + Send + Sync + 'static, T: Copy> {
	fn swatch_event(&self, value: T) -> E;
}

/// Swatch picker wired to menu events.
pub struct SwatchSelect<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SwatchEventMap<E, T> + Copy,
{
	pub swatch: SwatchSingleSelect<T>,
	pub map: M,
	_marker: core::marker::PhantomData<E>,
}

impl<E, T, M> SwatchSelect<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SwatchEventMap<E, T> + Copy,
{
	pub const fn new(swatch: SwatchSingleSelect<T>, map: M) -> Self {
		Self { swatch, map, _marker: core::marker::PhantomData }
	}
}

impl<E, T, M> RenderMenu for SwatchSelect<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
	M: SwatchEventMap<E, T> + Copy,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		SwatchPicker::new(self.swatch.value, self.map).render_with(renderer, parent, context);
	}
}

/// Inline swatch buttons without a field label.
pub struct SwatchPicker<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SwatchEventMap<E, T> + Copy,
{
	pub active: T,
	pub map: M,
	_marker: core::marker::PhantomData<E>,
}

impl<E, T, M> SwatchPicker<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SwatchEventMap<E, T> + Copy,
{
	pub const fn new(active: T, map: M) -> Self {
		Self { active, map, _marker: core::marker::PhantomData }
	}
}

impl<E, T, M> RenderMenu for SwatchPicker<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
	M: SwatchEventMap<E, T> + Copy,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
		parent.spawn((inline_chip_row(), Pickable::IGNORE)).with_children(|row| {
			for value in T::values() {
				let value = *value;
				let selected = value == self.active;
				row.spawn((
					Button,
					swatch_node(selected),
					BorderColor::all(if selected { Color::WHITE } else { MUTED }),
					BackgroundColor(color_from_hex(value.color_hex())),
					crate::widgets::MenuButton(self.map.swatch_event(value)),
				));
			}
		});
	}
}

impl<T> RenderMenu for SwatchSingleSelect<T>
where
	T: Copy + SwatchOption,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
		parent.spawn((
			swatch_node(true),
			BorderColor::all(Color::WHITE),
			BackgroundColor(color_from_hex(self.value.color_hex())),
		));
	}
}
