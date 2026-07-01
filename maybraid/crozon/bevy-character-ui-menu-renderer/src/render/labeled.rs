use bevy::prelude::*;
use character_ui_menu::{BlockLabeled, Labeled};

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crate::widgets::{row_node, text};

impl<T: RenderMenu> RenderMenu for Labeled<T> {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
			text(row, self.label, 11.0, Color::WHITE);
			self.value.render_with(renderer, row, context);
		});
	}
}

impl<T: RenderMenu> RenderMenu for BlockLabeled<T> {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		text(parent, self.label, 12.0, Color::srgb(0.78, 0.84, 0.92));
		self.value.render_with(renderer, parent, context);
	}
}
