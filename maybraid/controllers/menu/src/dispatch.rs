//! `MenuNavPad` → [`MenuNavImpulse`] on the focused menu.

use bevy::prelude::*;
use maybraid_input::{MenuNavImpulse, MenuNavPad};
use menu_components::{
	HudMenu, HudOverlayMenu, ShortTextModal, ShortTextPad, TextMenu, TextMenuInputLock,
};

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
	modal: Res<ShortTextModal>,
	nav: Res<MenuNavPad>,
	controllers: Query<&MenuController>,
	pads: Query<Entity, (With<ShortTextPad>, With<HudMenu>)>,
	mut commands: Commands,
) {
	if nav.events.is_empty() {
		return;
	}
	if modal.is_open() {
		let Some(pad) = pads.iter().next() else {
			return;
		};
		for event in &nav.events {
			commands.trigger(MenuNavImpulse::new(pad, *event));
		}
		return;
	}
	if lock.0 {
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
