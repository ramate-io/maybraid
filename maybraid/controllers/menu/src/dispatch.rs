//! `MenuNavPad` → [`MenuNavImpulse`] on the focused menu.

use bevy::prelude::*;
use maybraid_input::{MenuNavImpulse, MenuNavPad};
use menu_components::{HudMenu, HudOverlayMenu, TextMenu, TextMenuInputLock};

use crate::controller::MenuController;

pub fn refresh_menu_focus(
	children: Query<&Children>,
	overlays: Query<Entity, With<HudOverlayMenu>>,
	hud_menus: Query<Entity, (With<HudMenu>, Without<HudOverlayMenu>)>,
	text_menus: Query<Entity, With<TextMenu>>,
	mut controllers: Query<(Entity, &mut MenuController)>,
) {
	for (root, mut controller) in &mut controllers {
		controller.focus =
			MenuController::resolve(root, &children, &overlays, &hud_menus, &text_menus);
	}
}

pub fn dispatch_menu_nav(
	lock: Res<TextMenuInputLock>,
	nav: Res<MenuNavPad>,
	controllers: Query<&MenuController>,
	mut commands: Commands,
) {
	if lock.0 || nav.events.is_empty() {
		return;
	}
	for controller in &controllers {
		let Some(focus) = controller.focus else {
			continue;
		};
		for event in &nav.events {
			commands.trigger(MenuNavImpulse::new(focus, *event));
		}
	}
}
