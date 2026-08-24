use bevy::prelude::*;
use menu_components::{MenuComponentsPlugin, TextMenuSystems};
use std::marker::PhantomData;

use crate::event::CharacterMenuEvent;
use crate::input::{
	emit_hud_activate_on_click, emit_hud_activate_on_enter, emit_hud_focus,
	emit_overlay_close_on_click, emit_overlay_close_on_escape, emit_overlay_open_on_click,
};

/// Keyboard / observer input, then host UI rebuild.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharacterHudSystems {
	Input,
	Sync,
}

/// Registers leaf/overlay observers and [`CharacterMenuEvent`].
pub struct MaybraidCharacterMenuRendererPlugin<E>(PhantomData<fn() -> E>)
where
	E: Copy + Clone + Send + Sync + 'static;

impl<E> Default for MaybraidCharacterMenuRendererPlugin<E>
where
	E: Copy + Clone + Send + Sync + 'static,
{
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<E> Plugin for MaybraidCharacterMenuRendererPlugin<E>
where
	E: Copy + Clone + Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		app.configure_sets(
			Update,
			(
				CharacterHudSystems::Input.after(TextMenuSystems::Navigate),
				CharacterHudSystems::Sync.after(CharacterHudSystems::Input),
			),
		)
		.add_message::<CharacterMenuEvent<E>>()
		.add_observer(emit_hud_activate_on_click::<E>)
		.add_observer(emit_overlay_open_on_click)
		.add_observer(emit_overlay_close_on_click)
		.add_systems(
			Update,
			(emit_hud_focus::<E>, emit_hud_activate_on_enter::<E>, emit_overlay_close_on_escape)
				.in_set(CharacterHudSystems::Input),
		);
	}
}
