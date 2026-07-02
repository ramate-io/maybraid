use bevy::prelude::*;
use character_ui_menu::{Slider, SliderStep};

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{compact_control_row, render_button, text};

impl RenderMenu for Slider {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
		text(parent, &format!("{:.2}", self.value), 11.0, Color::srgb(0.85, 0.95, 1.0));
	}
}

impl<E> RenderMenu for SliderStep<E>
where
	E: Copy + Send + Sync + 'static,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		parent.spawn((compact_control_row(), Pickable::IGNORE)).with_children(|row| {
			render_button(row, "-", self.decrease, false);
			self.slider.render_with(renderer, row, context);
			render_button(row, "+", self.increase, false);
		});
	}
}
