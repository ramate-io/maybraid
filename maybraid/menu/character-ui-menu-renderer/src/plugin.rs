use bevy::prelude::*;
use menu_components::MenuComponentsPlugin;

use crate::event::CharacterMenuEvent;

/// Registers [`CharacterMenuEvent`] and shared HUD widget systems.
pub struct MaybraidCharacterMenuRendererPlugin<M>(std::marker::PhantomData<M>)
where
	M: Clone + Send + Sync + 'static;

impl<M> Default for MaybraidCharacterMenuRendererPlugin<M>
where
	M: Clone + Send + Sync + 'static,
{
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

impl<M> Plugin for MaybraidCharacterMenuRendererPlugin<M>
where
	M: Clone + Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		app.add_message::<CharacterMenuEvent<M>>();
	}
}
