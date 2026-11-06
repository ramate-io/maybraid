//! Pointer and keyboard → [`MenuFocus`] / [`MenuActivate`] / overlay events.

use bevy::prelude::*;
use menu_components::ScopedMenuSession;
use menu_components::{
	HudMenu, HudMenuItem, HudMenuNavLocked, HudOverlayMenu, KeyboardMenuNav, MenuActivate,
	MenuFocus, MenuModalBackHold, MenuModalDismiss, MenuModalId, ReorderMark, TextMenuInputLock,
};
use menu_components::{MenuAction, MenuActionImpulse};
use menu_components::{MenuNav, MenuNavImpulse, MenuNavPad};

use crate::event::{OverlayClose, OverlayOpen};
use crate::widgets::{
	CloseOverlaySelect, EquippedOrderControls, InventoryFocusHint, InventoryInteraction,
	InventoryItemKey, InventoryMenuSession, MenuButton, OpenSelectKey, OverlaySelectRoot,
};

pub fn emit_hud_activate_on_click<E: Copy + Send + Sync + 'static>(
	click: On<PointerClick>,
	lock: Res<TextMenuInputLock>,
	buttons: Query<&MenuButton<E>>,
	keys: Query<&InventoryItemKey>,
	items: Query<&HudMenuItem>,
	mut menus: Query<(&mut HudMenu, Option<&mut InventoryMenuSession>)>,
	scoped: Query<(), With<ScopedMenuSession>>,
	mut interaction: ResMut<InventoryInteraction>,
	mut focus: ResMut<InventoryFocusHint>,
	mut commands: Commands,
) {
	let menu = items.get(click.entity).ok().map(|item| item.menu);
	if lock.0 && !menu.is_some_and(|menu| scoped.contains(menu)) {
		return;
	}
	let Ok(button) = buttons.get(click.entity) else {
		return;
	};
	if let Some(menu_entity) = menu {
		if let Ok((mut menu, state)) = menus.get_mut(menu_entity) {
			if let Ok(item) = items.get(click.entity) {
				menu.selected = item.index;
			}
			if let Some(mut state) = state {
				let state = &mut *state;
				if intercept_equipped_activate(
					click.entity,
					Some(menu_entity),
					&keys,
					&mut state.interaction,
					&mut state.focus,
					&mut commands,
				) {
					return;
				}
				commands.trigger(MenuActivate { entity: click.entity, choice: button.0 });
				return;
			}
			if scoped.contains(menu_entity) {
				let mut state = InventoryMenuSession::default();
				let intercepted = intercept_equipped_activate(
					click.entity,
					Some(menu_entity),
					&keys,
					&mut state.interaction,
					&mut state.focus,
					&mut commands,
				);
				commands.entity(menu_entity).try_insert(state);
				if !intercepted {
					commands.trigger(MenuActivate { entity: click.entity, choice: button.0 });
				}
				return;
			}
		}
	}
	if intercept_equipped_activate(
		click.entity,
		menu,
		&keys,
		&mut interaction,
		&mut focus,
		&mut commands,
	) {
		return;
	}
	commands.trigger(MenuActivate { entity: click.entity, choice: button.0 });
}

pub fn emit_overlay_open_on_click(
	click: On<PointerClick>,
	lock: Res<TextMenuInputLock>,
	keys: Query<&OpenSelectKey>,
	scoped: Query<(), With<ScopedMenuSession>>,
	mut commands: Commands,
) {
	if lock.0 && !scoped.contains(click.entity) {
		return;
	}
	let Ok(key) = keys.get(click.entity) else {
		return;
	};
	commands.trigger(OverlayOpen { entity: click.entity, key: key.0 });
}

pub fn emit_overlay_close_on_click(
	click: On<PointerClick>,
	lock: Res<TextMenuInputLock>,
	closes: Query<(), With<CloseOverlaySelect>>,
	scoped: Query<(), With<ScopedMenuSession>>,
	mut commands: Commands,
) {
	if (lock.0 && !scoped.contains(click.entity)) || closes.get(click.entity).is_err() {
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
	overlay_menus: Query<(Entity, &HudMenu), (With<HudOverlayMenu>, Without<ScopedMenuSession>)>,
	panel_menus: Query<(Entity, &HudMenu), (Without<HudOverlayMenu>, Without<ScopedMenuSession>)>,
	items: Query<(
		Entity,
		&HudMenuItem,
		Option<&MenuButton<E>>,
		Option<&OpenSelectKey>,
		Option<&CloseOverlaySelect>,
	)>,
	keys: Query<&InventoryItemKey>,
	mut interaction: ResMut<InventoryInteraction>,
	mut focus: ResMut<InventoryFocusHint>,
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
	activate_selected_hud_item(
		None,
		menu_entity,
		menu,
		&items,
		&keys,
		&mut interaction,
		&mut focus,
		&mut commands,
	);
}

pub fn emit_hud_activate_on_nav<E: Copy + Send + Sync + 'static>(
	impulse: On<MenuNavImpulse>,
	mut menus: Query<(&HudMenu, Option<&mut InventoryMenuSession>)>,
	scoped: Query<(), With<ScopedMenuSession>>,
	items: Query<(
		Entity,
		&HudMenuItem,
		Option<&MenuButton<E>>,
		Option<&OpenSelectKey>,
		Option<&CloseOverlaySelect>,
	)>,
	keys: Query<&InventoryItemKey>,
	mut interaction: ResMut<InventoryInteraction>,
	mut focus: ResMut<InventoryFocusHint>,
	mut commands: Commands,
) {
	if impulse.event().nav != MenuNav::Select {
		return;
	}
	let Ok((menu, state)) = menus.get_mut(impulse.entity) else {
		return;
	};
	if let Some(mut state) = state {
		let state = &mut *state;
		activate_selected_hud_item(
			Some(impulse.event()),
			impulse.entity,
			menu,
			&items,
			&keys,
			&mut state.interaction,
			&mut state.focus,
			&mut commands,
		);
		return;
	}
	if scoped.contains(impulse.entity) {
		let mut state = InventoryMenuSession::default();
		activate_selected_hud_item(
			Some(impulse.event()),
			impulse.entity,
			menu,
			&items,
			&keys,
			&mut state.interaction,
			&mut state.focus,
			&mut commands,
		);
		commands.entity(impulse.entity).try_insert(state);
		return;
	}
	activate_selected_hud_item(
		Some(impulse.event()),
		impulse.entity,
		menu,
		&items,
		&keys,
		&mut interaction,
		&mut focus,
		&mut commands,
	);
}

fn activate_selected_hud_item<E: Copy + Send + Sync + 'static>(
	impulse: Option<&MenuNavImpulse>,
	menu_entity: Entity,
	menu: &HudMenu,
	items: &Query<(
		Entity,
		&HudMenuItem,
		Option<&MenuButton<E>>,
		Option<&OpenSelectKey>,
		Option<&CloseOverlaySelect>,
	)>,
	keys: &Query<&InventoryItemKey>,
	interaction: &mut InventoryInteraction,
	focus: &mut InventoryFocusHint,
	commands: &mut Commands,
) {
	for (entity, item, button, open, close) in items.iter() {
		if item.menu != menu_entity || item.index != menu.selected {
			continue;
		}
		if intercept_equipped_activate(
			entity,
			Some(menu_entity),
			keys,
			interaction,
			focus,
			commands,
		) {
			break;
		}
		if let Some(button) = button {
			if let Some(impulse) = impulse {
				impulse.deliver(commands, MenuActivate { entity, choice: button.0 });
			} else {
				commands.trigger(MenuActivate { entity, choice: button.0 });
			}
		}
		if let Some(open) = open {
			if let Some(impulse) = impulse {
				impulse.deliver(commands, OverlayOpen { entity, key: open.0 });
			} else {
				commands.trigger(OverlayOpen { entity, key: open.0 });
			}
		}
		if close.is_some() {
			if let Some(impulse) = impulse {
				impulse.deliver(commands, OverlayClose { entity });
			} else {
				commands.trigger(OverlayClose { entity });
			}
		}
		break;
	}
}

fn intercept_equipped_activate(
	entity: Entity,
	menu: Option<Entity>,
	keys: &Query<&InventoryItemKey>,
	interaction: &mut InventoryInteraction,
	focus: &mut InventoryFocusHint,
	commands: &mut Commands,
) -> bool {
	let Ok(key) = keys.get(entity) else {
		exit_reorder(interaction, menu, commands);
		return false;
	};
	if !key.equipped {
		exit_reorder(interaction, menu, commands);
		return false;
	}
	focus.0 = Some(*key);
	let same = matches!(*interaction, InventoryInteraction::Reorder { item_index } if item_index == key.index);
	if same {
		exit_reorder(interaction, menu, commands);
		return true;
	}
	*interaction = InventoryInteraction::Reorder { item_index: key.index };
	if let Some(menu) = menu {
		commands.entity(menu).try_insert(HudMenuNavLocked);
	}
	true
}

fn exit_reorder(
	interaction: &mut InventoryInteraction,
	menu: Option<Entity>,
	commands: &mut Commands,
) {
	if interaction.reordering().is_none() {
		return;
	}
	*interaction = InventoryInteraction::Browse;
	if let Some(menu) = menu {
		commands.entity(menu).try_remove::<HudMenuNavLocked>();
	}
}

pub fn emit_inventory_reorder_nav<E: Copy + Send + Sync + 'static>(
	impulse: On<MenuNavImpulse>,
	interaction: Res<InventoryInteraction>,
	menus: Query<Option<&InventoryMenuSession>, With<HudMenuNavLocked>>,
	items: Query<(Entity, &HudMenuItem, &InventoryItemKey, &EquippedOrderControls<E>)>,
	mut commands: Commands,
) {
	let Ok(state) = menus.get(impulse.entity) else {
		return;
	};
	let Some(item_index) = state.map_or(*interaction, |state| state.interaction).reordering()
	else {
		return;
	};
	// Hover/focus is positional and can change while the host rebuilds the
	// tiles. Reorder always belongs to the bag item that entered this mode.
	let Some((entity, _, _, controls)) = items.iter().find(|(_, item, key, _)| {
		item.menu == impulse.entity && key.equipped && key.index == item_index
	}) else {
		return;
	};
	let event = match impulse.event().nav {
		MenuNav::Left => controls.move_left,
		MenuNav::Right => controls.move_right,
		_ => None,
	};
	if let Some(event) = event {
		impulse.deliver(&mut commands, MenuActivate { entity, choice: event });
	}
}

/// Standalone renderers use keyboard edges; controller hosts disable this path
/// and supply the same impulses through their repeat/deadzone controller.
pub fn emit_inventory_reorder_keyboard(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	stack: Res<menu_components::MenuModalStack>,
	roots: Query<(Entity, &menu_components::MenuModal)>,
	children: Query<&ChildOf>,
	menus: Query<Entity, (With<HudMenu>, With<HudMenuNavLocked>, Without<ScopedMenuSession>)>,
	mut commands: Commands,
) {
	if !keyboard_nav.is_enabled() || lock.0 {
		return;
	}
	let nav = if keyboard.just_pressed(KeyCode::ArrowLeft) {
		MenuNav::Left
	} else if keyboard.just_pressed(KeyCode::ArrowRight) {
		MenuNav::Right
	} else {
		return;
	};
	for entity in &menus {
		if !menu_components::entity_under_nav_owner(entity, &stack, &roots, &children) {
			continue;
		}
		commands.trigger(MenuNavImpulse::new(entity, nav));
	}
}

pub fn consume_inventory_reorder_back(
	mut interaction: ResMut<InventoryInteraction>,
	mut hold: ResMut<MenuModalBackHold>,
	mut nav: ResMut<MenuNavPad>,
	keyboard: Res<ButtonInput<KeyCode>>,
	menus: Query<Entity, (With<HudMenu>, With<HudOverlayMenu>)>,
	mut commands: Commands,
) {
	if interaction.reordering().is_none() {
		return;
	}
	let back = nav.just_pressed(MenuNav::Back) || keyboard.just_pressed(KeyCode::Escape);
	if !back {
		return;
	}
	*interaction = InventoryInteraction::Browse;
	hold.0 = true;
	nav.events.retain(|event| *event != MenuNav::Back);
	for entity in &menus {
		commands.entity(entity).try_remove::<HudMenuNavLocked>();
	}
}

pub fn emit_inventory_unequip_on_action<E: Copy + Send + Sync + 'static>(
	impulse: On<MenuActionImpulse>,
	mut overlay_menus: Query<
		(Entity, &HudMenu, Option<&mut InventoryMenuSession>),
		With<HudOverlayMenu>,
	>,
	items: Query<(Entity, &HudMenuItem, Option<&EquippedOrderControls<E>>)>,
	mut interaction: ResMut<InventoryInteraction>,
	scoped: Query<(), With<ScopedMenuSession>>,
	mut commands: Commands,
) {
	if impulse.action != MenuAction::Unequip {
		return;
	}
	let Ok((menu_entity, menu, state)) = overlay_menus.get_mut(impulse.entity) else {
		return;
	};
	let Some((entity, _, Some(controls))) = items
		.iter()
		.find(|(_, item, _)| item.menu == menu_entity && item.index == menu.selected)
	else {
		return;
	};
	let Some(event) = controls.unequip else {
		return;
	};
	if let Some(mut state) = state {
		state.interaction = InventoryInteraction::Browse;
	} else if scoped.contains(menu_entity) {
		commands.entity(menu_entity).try_insert(InventoryMenuSession::default());
	} else {
		*interaction = InventoryInteraction::Browse;
	}
	commands.entity(menu_entity).try_remove::<HudMenuNavLocked>();
	impulse.deliver(&mut commands, MenuActivate { entity, choice: event });
}

pub fn mark_reorder_tiles(
	interaction: Res<InventoryInteraction>,
	tiles: Query<(Entity, &HudMenuItem, &InventoryItemKey, Option<&ReorderMark>)>,
	sessions: Query<&InventoryMenuSession>,
	scoped: Query<(), With<ScopedMenuSession>>,
	mut commands: Commands,
) {
	for (entity, item, key, mark) in &tiles {
		let selected = sessions
			.get(item.menu)
			.map(|s| s.interaction)
			.unwrap_or_else(|_| {
				if scoped.contains(item.menu) {
					InventoryInteraction::Browse
				} else {
					*interaction
				}
			})
			.reordering();
		let want = selected == Some(key.index) && key.equipped;
		match (want, mark.is_some()) {
			(true, false) => {
				commands.entity(entity).insert(ReorderMark);
			}
			(false, true) => {
				commands.entity(entity).remove::<ReorderMark>();
			}
			_ => {}
		}
	}
}

pub fn close_overlay_on_modal_dismiss(
	mut dismiss: MessageReader<MenuModalDismiss>,
	roots: Query<Entity, With<OverlaySelectRoot>>,
	mut commands: Commands,
) {
	if !dismiss.read().any(|event| event.is(MenuModalId::OVERLAY_SELECT)) {
		return;
	}
	let Ok(root) = roots.single() else {
		return;
	};
	commands.trigger(OverlayClose { entity: root });
}

#[cfg(test)]
mod tests {
	use super::{close_overlay_on_modal_dismiss, OverlaySelectRoot};
	use crate::event::OverlayClose as OverlayCloseEvent;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use menu_components::{
		consume_menu_modal_back, consume_screen_back, MenuModalDismiss, MenuModalId,
		MenuModalLayer, MenuModalStack, ScreenBackPressed,
	};
	use menu_components::{MenuNav, MenuNavPad};

	#[derive(Resource, Default)]
	struct OverlayCloseHits(u32);

	fn count_close(_close: On<OverlayCloseEvent>, mut hits: ResMut<OverlayCloseHits>) {
		hits.0 += 1;
	}

	fn overlay_back_world() -> World {
		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<ButtonInput<KeyCode>>();
		world.init_resource::<OverlayCloseHits>();
		world.init_resource::<Messages<MenuModalDismiss>>();
		world.init_resource::<Messages<ScreenBackPressed>>();
		world.add_observer(count_close);
		world.spawn(OverlaySelectRoot);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		world.resource_mut::<MenuNavPad>().events.push(MenuNav::Back);
		world.run_system_once(consume_menu_modal_back).expect("consume");
		world.run_system_once(close_overlay_on_modal_dismiss).expect("close");
		world.flush();
		world
	}

	#[test]
	fn back_closes_overlay_and_does_not_leave_the_screen() {
		let mut world = overlay_back_world();
		assert_eq!(world.resource::<OverlayCloseHits>().0, 1);
		assert_eq!(
			world
				.run_system_once(|mut dismiss: MessageReader<MenuModalDismiss>| {
					dismiss.read().next().map(|event| event.id)
				})
				.expect("dismiss"),
			Some(MenuModalId::OVERLAY_SELECT)
		);
		let left = world
			.run_system_once(
				|nav: Res<MenuNavPad>,
				 stack: Res<MenuModalStack>,
				 mut backs: MessageReader<ScreenBackPressed>| {
					consume_screen_back(&nav, &stack, &mut backs)
				},
			)
			.expect("screen back");
		assert!(!left);
	}

	#[test]
	fn escape_closes_overlay_without_nav_back() {
		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<ButtonInput<KeyCode>>();
		world.init_resource::<OverlayCloseHits>();
		world.init_resource::<Messages<MenuModalDismiss>>();
		world.add_observer(count_close);
		world.spawn(OverlaySelectRoot);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
		world.run_system_once(consume_menu_modal_back).expect("consume");
		world.run_system_once(close_overlay_on_modal_dismiss).expect("close");
		world.flush();
		assert_eq!(world.resource::<OverlayCloseHits>().0, 1);
	}

	#[test]
	fn reorder_back_keeps_the_overlay_open() {
		use super::consume_inventory_reorder_back;
		use crate::widgets::InventoryInteraction;
		use menu_components::MenuModalBackHold;

		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<ButtonInput<KeyCode>>();
		world.init_resource::<MenuModalBackHold>();
		world.init_resource::<InventoryInteraction>();
		world.init_resource::<Messages<MenuModalDismiss>>();
		*world.resource_mut::<InventoryInteraction>() =
			InventoryInteraction::Reorder { item_index: 3 };
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		world.resource_mut::<MenuNavPad>().events.push(MenuNav::Back);
		world.run_system_once(consume_inventory_reorder_back).expect("hold");
		world.run_system_once(consume_menu_modal_back).expect("consume");
		assert_eq!(*world.resource::<InventoryInteraction>(), InventoryInteraction::Browse);
		assert!(world.resource::<MenuModalBackHold>().0);
		let dismissed = world
			.run_system_once(|mut dismiss: MessageReader<MenuModalDismiss>| {
				dismiss.read().next().is_some()
			})
			.expect("dismiss");
		assert!(!dismissed);
	}

	#[derive(Resource, Default)]
	struct Unequipped(Vec<(Entity, u8)>);

	fn record_unequip(event: On<menu_components::MenuActivate<u8>>, mut hits: ResMut<Unequipped>) {
		hits.0.push((event.entity, event.choice));
	}

	#[test]
	fn unequip_only_uses_the_addressed_menus_selected_tile() -> anyhow::Result<()> {
		use crate::input::emit_inventory_unequip_on_action;
		use crate::widgets::{EquippedOrderControls, InventoryInteraction};
		use menu_components::{
			HudMenu, HudMenuItem, HudMenuNavLocked, HudOverlayMenu, TextMenuInputLock,
		};
		use menu_components::{MenuAction, MenuActionImpulse};

		let mut world = World::new();
		world.init_resource::<TextMenuInputLock>();
		world.init_resource::<InventoryInteraction>();
		world.init_resource::<Unequipped>();
		world.add_observer(emit_inventory_unequip_on_action::<u8>);
		world.add_observer(record_unequip);
		let first = world.spawn((HudMenu::new(1), HudOverlayMenu, HudMenuNavLocked)).id();
		let selected = world.spawn((HudMenu::new(2), HudOverlayMenu, HudMenuNavLocked)).id();
		world
			.get_mut::<HudMenu>(selected)
			.ok_or_else(|| anyhow::anyhow!("selected menu"))?
			.selected = 1;
		let controls = |value| EquippedOrderControls {
			unequip: Some(value),
			move_left: None,
			move_right: None,
		};
		world.spawn((HudMenuItem { menu: first, index: 0 }, controls(1u8)));
		world.spawn((HudMenuItem { menu: selected, index: 0 }, controls(2u8)));
		let tile = world.spawn((HudMenuItem { menu: selected, index: 1 }, controls(3u8))).id();
		*world.resource_mut::<InventoryInteraction>() =
			InventoryInteraction::Reorder { item_index: 7 };
		world.trigger(MenuActionImpulse::new(selected, MenuAction::Unequip));
		world.flush();
		assert_eq!(world.resource::<Unequipped>().0, vec![(tile, 3)]);
		assert_eq!(*world.resource::<InventoryInteraction>(), InventoryInteraction::Browse);
		assert!(world.get::<HudMenuNavLocked>(selected).is_none());
		assert!(world.get::<HudMenuNavLocked>(first).is_some());

		// A modal target, or a target removed during a screen swap, must not
		// fall back to the first inventory overlay.
		let modal = world.spawn(HudMenu::new(1)).id();
		for entity in [modal, Entity::PLACEHOLDER] {
			world.trigger(MenuActionImpulse::new(entity, MenuAction::Unequip));
		}
		world.flush();
		assert_eq!(world.resource::<Unequipped>().0.len(), 1);
		Ok(())
	}
}

/// Back exits reorder in only the addressed inventory session.
pub fn consume_scoped_inventory_reorder_back(
	mut impulse: On<MenuNavImpulse>,
	mut sessions: Query<&mut InventoryMenuSession>,
	mut commands: Commands,
) {
	if impulse.nav != MenuNav::Back {
		return;
	}
	let Ok(mut state) = sessions.get_mut(impulse.entity) else {
		return;
	};
	if state.interaction.reordering().is_none() {
		return;
	}
	state.interaction = InventoryInteraction::Browse;
	commands.entity(impulse.entity).try_remove::<HudMenuNavLocked>();
	impulse.propagate(false);
}

#[cfg(test)]
mod scoped_tests {
	use crate::{
		EquippedOrderControls, InventoryFocusHint, InventoryInteraction, InventoryItemKey,
		InventoryMenuSession,
	};
	use bevy::prelude::*;
	use menu_components::{
		HudMenu, HudMenuItem, HudMenuNavLocked, HudOverlayMenu, MenuAction, MenuActionImpulse,
		MenuActivate, MenuNav, MenuNavImpulse, ScopedMenuSession,
	};
	#[derive(Resource, Default)]
	struct Hits(Vec<(Entity, u8)>);
	fn record(event: On<MenuActivate<u8>>, mut hits: ResMut<Hits>) {
		hits.0.push((event.entity, event.choice));
	}
	#[test]
	fn independent_reorder_sessions_target_stable_items_and_cleanup_only_the_selected_menu(
	) -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<InventoryInteraction>();
		world.init_resource::<InventoryFocusHint>();
		world.init_resource::<Hits>();
		world.add_observer(super::emit_inventory_reorder_nav::<u8>);
		world.add_observer(super::emit_inventory_unequip_on_action::<u8>);
		world.add_observer(record);
		*world.resource_mut::<InventoryInteraction>() =
			InventoryInteraction::Reorder { item_index: 99 };
		let session = || InventoryMenuSession {
			interaction: InventoryInteraction::Reorder { item_index: 7 },
			..default()
		};
		let first = world
			.spawn((
				HudMenu::new(2),
				HudOverlayMenu,
				HudMenuNavLocked,
				ScopedMenuSession,
				session(),
			))
			.id();
		let second = world
			.spawn((
				HudMenu::new(2),
				HudOverlayMenu,
				HudMenuNavLocked,
				ScopedMenuSession,
				session(),
			))
			.id();
		let controls = |choice| EquippedOrderControls {
			unequip: Some(choice),
			move_left: Some(choice + 1),
			move_right: None,
		};
		world.spawn((
			HudMenuItem { menu: first, index: 1 },
			InventoryItemKey { index: 7, equipped: true },
			controls(10u8),
		));
		let stable = world
			.spawn((
				HudMenuItem { menu: second, index: 1 },
				InventoryItemKey { index: 7, equipped: true },
				controls(20u8),
			))
			.id();
		let selected = world.spawn((HudMenuItem { menu: second, index: 0 }, controls(30u8))).id();
		world.trigger(MenuNavImpulse::new(second, MenuNav::Left));
		world.flush();
		assert_eq!(world.resource::<Hits>().0, [(stable, 21)]);
		world.trigger(MenuActionImpulse::new(second, MenuAction::Unequip));
		world.flush();
		assert_eq!(world.resource::<Hits>().0.last(), Some(&(selected, 30)));
		assert!(world
			.get::<InventoryMenuSession>(second)
			.is_some_and(|s| s.interaction == InventoryInteraction::Browse));
		assert!(world
			.get::<InventoryMenuSession>(first)
			.is_some_and(|s| s.interaction.reordering() == Some(7)));
		assert!(world.get::<HudMenuNavLocked>(first).is_some());
		assert!(world.get::<HudMenuNavLocked>(second).is_none());
		assert_eq!(world.resource::<InventoryInteraction>().reordering(), Some(99));
		Ok(())
	}
}
