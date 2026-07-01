use bevy::prelude::*;
use character_ui_menu::{LabelOption, ListValues, SingleSelect};

use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};

impl<T> RenderMenu for SingleSelect<T>
where
	T: Copy + LabelOption + ListValues,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		_renderer: &Renderer,
		_parent: &mut ChildSpawnerCommands,
		_context: &mut RenderContext<'_, C>,
	) {
	}
}
