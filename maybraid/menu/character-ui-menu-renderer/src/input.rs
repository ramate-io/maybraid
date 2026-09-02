//! Pointer and keyboard → [`MenuFocus`] / [`MenuActivate`] / overlay events.

use bevy::prelude::*;
use maybraid_input::{MenuNav, MenuNavImpulse};
use menu_components::{
	HudMenu, HudMenuItem, HudOverlayMenu, KeyboardMenuNav, MenuActivate, MenuFocus,
	TextMenuInputLock,
};

use crate::event::{OverlayClose, OverlayOpen};
use crate::widgets::{CloseOverlaySelect, MenuButton, OpenSelectKey, OverlaySelectRoot};

pub fn emit_hud_activate_on_click<E: Copy + Send + Sync + 'static>(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	buttons: Query<&MenuButton<E>>,
	mut commands: Commands,
) {
	if lock.0 {
		return;
	}
	let Ok(button) = buttons.get(click.entity) else {
		return;
	};
	commands.trigger(MenuActivate { entity: click.entity, choice: button.0 });
}

pub fn emit_overlay_open_on_click(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	keys: Query<&OpenSelectKey>,
	mut commands: Commands,
) {
	if lock.0 {
		return;
	}
	let Ok(key) = keys.get(click.entity) else {
		return;
	};
	commands.trigger(OverlayOpen { entity: click.entity, key: key.0 });
}

pub fn emit_overlay_close_on_click(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	closes: Query<(), With<CloseOverlaySelect>>,
	mut commands: Commands,
) {
	if lock.0 || closes.get(click.entity).is_err() {
		return;
	}
	commands.trigger(OverlayClose { entity: click.entity });
}

pub fn emit_hud_focus<E: Copy + Send + Sync + 'static>(
	menus: Query<(Entity, &HudMenu), Changed<HudMenu>>,
	items: Query<(&HudMenuItem, &MenuButton<E>)>,
	mut commands: Commands,
) {
	for (menu_entity, menu) in &menus {
		if let Some((_, button)) = items
			.iter()
			.find(|(item, _)| item.menu == menu_entity && item.index == menu.selected)
		{
			commands.trigger(MenuFocus { entity: menu_entity, choice: button.0 });
		}
	}
}

pub fn emit_hud_activate_on_enter<E: Copy + Send + Sync + 'static>(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	overlay_menus: Query<(Entity, &HudMenu), With<HudOverlayMenu>>,
	panel_menus: Query<(Entity, &HudMenu), Without<HudOverlayMenu>>,
	items: Query<(
		Entity,
		&HudMenuItem,
		Option<&MenuButton<E>>,
		Option<&OpenSelectKey>,
		Option<&CloseOverlaySelect>,
	)>,
	mut commands: Commands,
) {
	if !keyboard_nav.is_enabled() || lock.0 || !keyboard.just_pressed(KeyCode::Enter) {
		return;
	}
	let Some((menu_entity, menu)) =
		overlay_menus.iter().next().or_else(|| panel_menus.iter().next())
	else {
		return;
	};
	activate_selected_hud_item(menu_entity, menu, &items, &mut commands);
}

pub fn emit_hud_activate_on_nav<E: Copy + Send + Sync + 'static>(
	impulse: On<MenuNavImpulse>,
	lock: Res<TextMenuInputLock>,
	menus: Query<&HudMenu>,
	items: Query<(
		Entity,
		&HudMenuItem,
		Option<&MenuButton<E>>,
		Option<&OpenSelectKey>,
		Option<&CloseOverlaySelect>,
	)>,
	mut commands: Commands,
) {
	if lock.0 || impulse.event().nav != MenuNav::Select {
		return;
	}
	let Ok(menu) = menus.get(impulse.entity) else {
		return;
	};
	activate_selected_hud_item(impulse.entity, menu, &items, &mut commands);
}

fn activate_selected_hud_item<E: Copy + Send + Sync + 'static>(
	menu_entity: Entity,
	menu: &HudMenu,
	items: &Query<(
		Entity,
		&HudMenuItem,
		Option<&MenuButton<E>>,
		Option<&OpenSelectKey>,
		Option<&CloseOverlaySelect>,
	)>,
	commands: &mut Commands,
) {
	for (entity, item, button, open, close) in items.iter() {
		if item.menu != menu_entity || item.index != menu.selected {
			continue;
		}
		if let Some(button) = button {
			commands.trigger(MenuActivate { entity, choice: button.0 });
		}
		if let Some(open) = open {
			commands.trigger(OverlayOpen { entity, key: open.0 });
		}
		if close.is_some() {
			commands.trigger(OverlayClose { entity });
		}
		break;
	}
}

pub fn emit_overlay_close_on_escape(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	roots: Query<Entity, With<OverlaySelectRoot>>,
	mut commands: Commands,
) {
	if !keyboard_nav.is_enabled() || lock.0 || !keyboard.just_pressed(KeyCode::Escape) {
		return;
	}
	let Ok(root) = roots.single() else {
		return;
	};
	commands.trigger(OverlayClose { entity: root });
}

pub fn emit_overlay_close_on_nav(
	impulse: On<MenuNavImpulse>,
	lock: Res<TextMenuInputLock>,
	roots: Query<Entity, With<OverlaySelectRoot>>,
	mut commands: Commands,
) {
	if lock.0 || impulse.event().nav != MenuNav::Back {
		return;
	}
	let Ok(root) = roots.single() else {
		return;
	};
	commands.trigger(OverlayClose { entity: root });
}
