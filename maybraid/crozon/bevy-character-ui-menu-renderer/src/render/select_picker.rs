use bevy::prelude::*;
use character_ui_menu::{LabelOption, ListValues, SingleSelect};

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{inline_chip_row, select_tile_node, tile_text, ACTIVE, INACTIVE};

pub trait SelectEventMap<E: Copy + Send + Sync + 'static, T: Copy> {
	fn select_event(&self, value: T) -> E;
}

/// Button row for picking among listed values.
pub struct SelectPicker<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SelectEventMap<E, T> + Copy,
{
	pub select: SingleSelect<T>,
	pub map: M,
	_marker: core::marker::PhantomData<E>,
}

impl<E, T, M> SelectPicker<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SelectEventMap<E, T> + Copy,
{
	pub const fn new(select: SingleSelect<T>, map: M) -> Self {
		Self { select, map, _marker: core::marker::PhantomData }
	}
}

impl<E, T, M> RenderMenu for SelectPicker<E, T, M>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues,
	M: SelectEventMap<E, T> + Copy,
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
				let active = value == self.select.value;
				row
					.spawn((
						Button,
						select_tile_node(),
						BackgroundColor(if active { ACTIVE } else { INACTIVE }),
						crate::widgets::MenuButton(self.map.select_event(value)),
					))
					.with_children(|button| {
						tile_text(button, value.label(), 9.0, Color::WHITE);
					});
			}
		});
	}
}
