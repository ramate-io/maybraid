use bevy::prelude::*;
use character_ui_menu::{Cycle, LabelOption};

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{compact_control_row, render_button, text};

impl<E, T> RenderMenu for Cycle<E, T>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + LabelOption,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
		parent.spawn((compact_control_row(), Pickable::IGNORE)).with_children(|row| {
			render_button(row, "<", self.minus, false);
			text(row, self.select.value.label(), 11.0, Color::srgb(0.85, 0.95, 1.0));
			render_button(row, ">", self.plus, false);
		});
	}
}
