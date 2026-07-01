use bevy::prelude::*;
use character_ui_menu::Slider;

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{render_button, row_node, text};

/// Labeled slider row: `-` value `+`.
pub struct LabeledSlider<E: Copy + Send + Sync + 'static> {
	pub label: &'static str,
	pub slider: Slider,
	pub decrease: E,
	pub increase: E,
}

impl<E: Copy + Send + Sync + 'static> RenderMenu for LabeledSlider<E> {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
		parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
			text(row, self.label, 11.0, Color::WHITE);
			render_button(row, "-", self.decrease, false);
			text(row, &format!("{:.2}", self.slider.value), 11.0, Color::srgb(0.85, 0.95, 1.0));
			render_button(row, "+", self.increase, false);
		});
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
