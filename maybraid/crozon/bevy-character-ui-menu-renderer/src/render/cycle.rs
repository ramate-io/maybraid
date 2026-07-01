use bevy::prelude::*;

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{render_button, row_node, text};

/// Labeled cycle picker: `<` value `>`.
pub struct LabeledCycle<E: Copy + Send + Sync + 'static> {
	pub label: &'static str,
	pub value_label: &'static str,
	pub minus: E,
	pub plus: E,
}

impl<E: Copy + Send + Sync + 'static> RenderMenu for LabeledCycle<E> {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
		parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
			text(row, self.label, 11.0, Color::WHITE);
			render_button(row, "<", self.minus, false);
			text(row, self.value_label, 11.0, Color::srgb(0.85, 0.95, 1.0));
			render_button(row, ">", self.plus, false);
		});
	}
}
