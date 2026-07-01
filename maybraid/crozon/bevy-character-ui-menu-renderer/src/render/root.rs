use bevy::prelude::*;
use character_ui_menu::Root;

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};

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
