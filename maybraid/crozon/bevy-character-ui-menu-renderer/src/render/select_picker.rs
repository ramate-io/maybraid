use bevy::prelude::*;
use character_ui_menu::{LabelOption, ListValues, SingleSelect};

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{text, ACTIVE, INACTIVE};

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
		for value in T::values() {
			let value = *value;
			let active = value == self.select.value;
			parent
				.spawn((
					Button,
					Node {
						min_width: Val::Px(28.0),
						height: Val::Px(22.0),
						padding: UiRect::axes(Val::Px(7.0), Val::Px(2.0)),
						justify_content: JustifyContent::Center,
						align_items: AlignItems::Center,
						..default()
					},
					BackgroundColor(if active { ACTIVE } else { INACTIVE }),
					crate::widgets::MenuButton(self.map.select_event(value)),
				))
				.with_children(|button| {
					text(button, value.label(), 10.0, Color::WHITE);
				});
		}
	}
}
