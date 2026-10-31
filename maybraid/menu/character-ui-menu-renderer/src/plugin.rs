use bevy::prelude::*;
use menu_components::{
	restore_short_text_editing, MenuComponentsPlugin, MenuModalSystems, TextMenuSystems,
};
use std::marker::PhantomData;

use crate::event::CharacterMenuEvent;
use crate::input::{
	close_overlay_on_modal_dismiss, consume_inventory_reorder_back, emit_hud_activate_on_click,
	emit_hud_activate_on_enter, emit_hud_activate_on_nav, emit_hud_focus,
	emit_inventory_reorder_keyboard, emit_inventory_reorder_nav, emit_inventory_unequip_on_action,
	emit_overlay_close_on_click, emit_overlay_open_on_click, mark_reorder_tiles,
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
		.init_resource::<crate::InventoryInteraction>()
		.init_resource::<crate::InventoryFocusHint>()
		.add_message::<CharacterMenuEvent<E>>()
		.add_observer(emit_hud_activate_on_click::<E>)
		.add_observer(emit_hud_activate_on_nav::<E>)
		.add_observer(emit_inventory_reorder_nav::<E>)
		.add_observer(crate::input::consume_scoped_inventory_reorder_back)
		.add_observer(emit_inventory_unequip_on_action::<E>)
		.add_observer(crate::input::emit_inventory_control_on_click)
		.add_observer(crate::input::apply_inventory_navigation_impulse)
		.add_observer(crate::input::navigate_inventory_menu)
		.add_observer(crate::input::emit_inventory_control_on_nav)
		.add_observer(emit_overlay_open_on_click)
		.add_observer(emit_overlay_close_on_click)
		.add_systems(Update, close_overlay_on_modal_dismiss.in_set(MenuModalSystems::Apply))
		.add_systems(
			Update,
			consume_inventory_reorder_back
				.in_set(MenuModalSystems::Consume)
				.before(menu_components::consume_menu_modal_back),
		)
		.add_systems(
			Update,
			crate::input::consume_inventory_session_keyboard_back
				.in_set(MenuModalSystems::Consume)
				.before(menu_components::consume_menu_modal_back),
		)
		.add_systems(
			Update,
			(
				emit_inventory_reorder_keyboard,
				crate::input::emit_inventory_keyboard_navigation,
				crate::input::emit_inventory_control_on_enter,
				emit_hud_focus::<E>,
				emit_hud_activate_on_enter::<E>,
				mark_reorder_tiles,
			)
				.in_set(CharacterHudSystems::Input),
		)
		.add_systems(
			Update,
			(
				crate::input::sync_inventory_focus_from_menu,
				crate::input::sync_inventory_region_focus,
			)
				.chain()
				.after(CharacterHudSystems::Sync),
		)
		.add_systems(Update, restore_short_text_editing.after(CharacterHudSystems::Sync));
		app.add_systems(
			Update,
			crate::inventory::animate_inventory_carousel.after(CharacterHudSystems::Input),
		);
	}
}
