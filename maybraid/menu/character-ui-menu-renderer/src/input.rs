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
	CloseOverlaySelect, EquippedOrderControls, InventoryControl, InventoryFocusHint,
	InventoryInteraction, InventoryItemKey, InventoryMenuSession, MenuButton, OpenSelectKey,
	OverlaySelectRoot, OverlaySelectViewport,
};

fn inventory_session_ancestor(
	entity: Entity,
	parents: &Query<&ChildOf>,
	sessions: &Query<&InventoryMenuSession>,
) -> Option<Entity> {
	std::iter::once(entity)
		.chain(parents.iter_ancestors(entity))
		.find(|entity| sessions.contains(*entity))
}

fn inventory_modal_blocks_shell(stack: &menu_components::MenuModalStack) -> bool {
	stack.consumed()
		|| (stack.owns_nav()
			&& stack.nav_owner() != Some(menu_components::MenuModalId::OVERLAY_SELECT))
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InventoryPresentationSnapshot {
	section: character_ui_menu::InventorySection,
	region: character_ui_menu::InventoryRegion,
	drawer: character_ui_menu::InventoryDrawerMode,
	gallery_scroll: usize,
}

impl InventoryPresentationSnapshot {
	fn from_session(session: &InventoryMenuSession) -> Self {
		Self {
			section: session.section,
			region: session.region,
			drawer: session.drawer,
			gallery_scroll: session.active_memory().gallery_scroll,
		}
	}
}

/// One-frame request from keyboard grid navigation to reveal its new gallery row.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct InventoryKeyboardScrollIntent(usize);

fn apply_inventory_navigation(
	session: &mut InventoryMenuSession,
	action: menu_components::InventoryNavigation,
) {
	if session.interaction.reordering().is_some() {
		return;
	}
	match action {
		menu_components::InventoryNavigation::PreviousSection => {
			let index = session.section.index();
			session.section = character_ui_menu::InventorySection::ALL[(index
				+ character_ui_menu::InventorySection::ALL.len()
				- 1)
				% character_ui_menu::InventorySection::ALL.len()];
		}
		menu_components::InventoryNavigation::NextSection => {
			let index = session.section.index();
			session.section = character_ui_menu::InventorySection::ALL
				[(index + 1) % character_ui_menu::InventorySection::ALL.len()];
		}
		menu_components::InventoryNavigation::ToggleDrawer => {
			session.drawer = match session.drawer {
				character_ui_menu::InventoryDrawerMode::Rotary => {
					character_ui_menu::InventoryDrawerMode::Gallery
				}
				character_ui_menu::InventoryDrawerMode::Gallery => {
					character_ui_menu::InventoryDrawerMode::Rotary
				}
			};
		}
		menu_components::InventoryNavigation::ToggleRegion => {
			session.region = match session.region {
				character_ui_menu::InventoryRegion::Drawer => {
					character_ui_menu::InventoryRegion::Equipped
				}
				character_ui_menu::InventoryRegion::Equipped => {
					character_ui_menu::InventoryRegion::Drawer
				}
			};
		}
		menu_components::InventoryNavigation::FocusDrawer => {
			session.region = character_ui_menu::InventoryRegion::Drawer;
		}
	}
	session.focus.0 = session.active_memory().focused_item;
}

/// Clickable shell controls share the same state transitions as routed pad input.
pub fn emit_inventory_control_on_click(
	click: On<Pointer<Click>>,
	controls: Query<&InventoryControl>,
	parents: Query<&ChildOf>,
	mut sessions: Query<&mut InventoryMenuSession>,
	stack: Res<menu_components::MenuModalStack>,
) {
	if inventory_modal_blocks_shell(&stack) {
		return;
	}
	let Ok(control) = controls.get(click.entity) else { return };
	let Some(session_entity) = std::iter::once(click.entity)
		.chain(parents.iter_ancestors(click.entity))
		.find(|entity| sessions.contains(*entity))
	else {
		return;
	};
	if let Ok(mut session) = sessions.get_mut(session_entity) {
		apply_inventory_navigation(&mut session, control_navigation(control.0));
	}
}

fn control_navigation(
	action: character_ui_menu::InventoryControlAction,
) -> menu_components::InventoryNavigation {
	match action {
		character_ui_menu::InventoryControlAction::PreviousSection => {
			menu_components::InventoryNavigation::PreviousSection
		}
		character_ui_menu::InventoryControlAction::NextSection => {
			menu_components::InventoryNavigation::NextSection
		}
		character_ui_menu::InventoryControlAction::ToggleDrawer => {
			menu_components::InventoryNavigation::ToggleDrawer
		}
		character_ui_menu::InventoryControlAction::ToggleRegion => {
			menu_components::InventoryNavigation::ToggleRegion
		}
		character_ui_menu::InventoryControlAction::FocusDrawer => {
			menu_components::InventoryNavigation::FocusDrawer
		}
	}
}

pub fn emit_inventory_control_on_nav(
	mut impulse: On<MenuNavImpulse>,
	areas: Query<&crate::inventory::InventoryScreenArea>,
	menus: Query<&HudMenu>,
	items: Query<(&HudMenuItem, &InventoryControl)>,
	parents: Query<&ChildOf>,
	mut sessions: Query<&mut InventoryMenuSession>,
	stack: Res<menu_components::MenuModalStack>,
) {
	if impulse.event().nav != MenuNav::Select || inventory_modal_blocks_shell(&stack) {
		return;
	}
	let Ok(menu) = menus.get(impulse.entity) else { return };
	if areas.get(impulse.entity).is_err() {
		return;
	}
	let Some((_, control)) = items
		.iter()
		.find(|(item, _)| item.menu == impulse.entity && item.index == menu.selected)
	else {
		return;
	};
	let session_entity = std::iter::once(impulse.entity)
		.chain(parents.iter_ancestors(impulse.entity))
		.find(|entity| sessions.contains(*entity));
	let Some(session_entity) = session_entity else { return };
	if let Ok(mut session) = sessions.get_mut(session_entity) {
		apply_inventory_navigation(&mut session, control_navigation(control.0));
	}
	impulse.propagate(false);
}

pub fn emit_inventory_control_on_enter(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	stack: Res<menu_components::MenuModalStack>,
	roots: Query<(Entity, &menu_components::MenuModal)>,
	parents: Query<&ChildOf>,
	menus: Query<
		(Entity, &HudMenu),
		(With<menu_components::InventoryHudMenu>, With<HudOverlayMenu>, Without<ScopedMenuSession>),
	>,
	items: Query<(&HudMenuItem, &InventoryControl)>,
	mut sessions: Query<&mut InventoryMenuSession>,
) {
	if !keyboard_nav.is_enabled()
		|| lock.0
		|| !keyboard.just_pressed(KeyCode::Enter)
		|| inventory_modal_blocks_shell(&stack)
	{
		return;
	}
	for (menu_entity, menu) in &menus {
		if !menu_components::entity_under_nav_owner(menu_entity, &stack, &roots, &parents) {
			continue;
		}
		let Some((_, control)) = items
			.iter()
			.find(|(item, _)| item.menu == menu_entity && item.index == menu.selected)
		else {
			continue;
		};
		let session_entity = std::iter::once(menu_entity)
			.chain(parents.iter_ancestors(menu_entity))
			.find(|entity| sessions.contains(*entity));
		if let Some(session_entity) = session_entity {
			if let Ok(mut session) = sessions.get_mut(session_entity) {
				apply_inventory_navigation(&mut session, control_navigation(control.0));
			}
		}
	}
}

/// Apply an authorized controller shortcut only to the addressed inventory viewport.
pub fn apply_inventory_navigation_impulse(
	impulse: On<menu_components::InventoryNavigationImpulse>,
	parents: Query<&ChildOf>,
	sessions: Query<&InventoryMenuSession>,
	stack: Res<menu_components::MenuModalStack>,
	mut commands: Commands,
) {
	if inventory_modal_blocks_shell(&stack) {
		return;
	}
	let Some(session_entity) = inventory_session_ancestor(impulse.entity, &parents, &sessions)
	else {
		return;
	};
	let action = impulse.action;
	impulse.queue(&mut commands, move |world| {
		if let Some(mut session) = world.get_mut::<InventoryMenuSession>(session_entity) {
			apply_inventory_navigation(&mut session, action);
		}
	});
}

/// Local keyboard path for renderers without a routed controller host.
pub fn emit_inventory_keyboard_navigation(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	stack: Res<menu_components::MenuModalStack>,
	roots: Query<(Entity, &menu_components::MenuModal)>,
	parents: Query<&ChildOf>,
	menus: Query<
		(Entity, &crate::inventory::InventoryScreenArea),
		(With<menu_components::InventoryHudMenu>, With<HudOverlayMenu>, Without<ScopedMenuSession>),
	>,
	mut sessions: Query<&mut InventoryMenuSession>,
	interaction: Res<InventoryInteraction>,
	mut commands: Commands,
) {
	if !keyboard_nav.is_enabled() || lock.0 || inventory_modal_blocks_shell(&stack) {
		return;
	}
	let action = if keyboard.just_pressed(KeyCode::PageUp) {
		Some(menu_components::InventoryNavigation::PreviousSection)
	} else if keyboard.just_pressed(KeyCode::PageDown) {
		Some(menu_components::InventoryNavigation::NextSection)
	} else if keyboard.just_pressed(KeyCode::KeyG) {
		Some(menu_components::InventoryNavigation::ToggleDrawer)
	} else if keyboard.just_pressed(KeyCode::Tab) {
		Some(menu_components::InventoryNavigation::ToggleRegion)
	} else {
		None
	};
	if let Some(action) = action {
		let mut visited = std::collections::HashSet::new();
		for (menu, _) in &menus {
			if !menu_components::entity_under_nav_owner(menu, &stack, &roots, &parents) {
				continue;
			}
			let Some(session_entity) = std::iter::once(menu)
				.chain(parents.iter_ancestors(menu))
				.find(|entity| sessions.contains(*entity))
			else {
				continue;
			};
			if !visited.insert(session_entity) {
				continue;
			}
			if let Ok(mut session) = sessions.get_mut(session_entity) {
				apply_inventory_navigation(&mut session, action);
			}
		}
		return;
	}
	if interaction.reordering().is_some() {
		return;
	}
	let nav = if keyboard.just_pressed(KeyCode::ArrowUp) {
		Some(MenuNav::Up)
	} else if keyboard.just_pressed(KeyCode::ArrowDown) {
		Some(MenuNav::Down)
	} else if keyboard.just_pressed(KeyCode::ArrowLeft) {
		Some(MenuNav::Left)
	} else if keyboard.just_pressed(KeyCode::ArrowRight) {
		Some(MenuNav::Right)
	} else {
		None
	};
	let Some(nav) = nav else { return };
	for (menu, _) in &menus {
		if !menu_components::entity_under_nav_owner(menu, &stack, &roots, &parents) {
			continue;
		}
		let Some(session_entity) = std::iter::once(menu)
			.chain(parents.iter_ancestors(menu))
			.find(|entity| sessions.contains(*entity))
		else {
			continue;
		};
		if sessions
			.get_mut(session_entity)
			.is_ok_and(|session| session.interaction.reordering().is_some())
		{
			continue;
		}
		commands.trigger(MenuNavImpulse::new(menu, nav));
	}
}

/// Inventory arrows use stable catalog ordinals rather than UI geometry.
pub fn navigate_inventory_menu(
	mut impulse: On<MenuNavImpulse>,
	areas: Query<&crate::inventory::InventoryScreenArea>,
	parents: Query<&ChildOf>,
	mut menus: Query<&mut HudMenu>,
	items: Query<(
		&HudMenuItem,
		Option<&crate::inventory::InventoryTile>,
		Option<&crate::inventory::InventoryAIOPSlot>,
		Option<&InventoryItemKey>,
	)>,
	mut sessions: Query<&mut InventoryMenuSession>,
	stack: Res<menu_components::MenuModalStack>,
	mut commands: Commands,
) {
	let Ok(area) = areas.get(impulse.entity) else { return };
	// Inventory arrows belong to this list and must not be re-applied by
	// observers on its viewport ancestors.
	impulse.propagate(false);
	let session_entity = std::iter::once(impulse.entity)
		.chain(parents.iter_ancestors(impulse.entity))
		.find(|entity| sessions.contains(*entity));
	let Some(session_entity) = session_entity else { return };
	let Ok(mut session) = sessions.get_mut(session_entity) else { return };
	if inventory_modal_blocks_shell(&stack) || session.interaction.reordering().is_some() {
		return;
	}
	let nav = impulse.event().nav;
	if !matches!(nav, MenuNav::Up | MenuNav::Down | MenuNav::Left | MenuNav::Right) {
		return;
	}
	let Ok(mut menu) = menus.get_mut(impulse.entity) else { return };
	let count = match area.0 {
		character_ui_menu::InventoryRegion::Drawer => items
			.iter()
			.filter(|(item, tile, _, _)| {
				item.menu == impulse.entity
					&& tile.is_some_and(|tile| {
						tile.section == session.section
							&& tile.region == character_ui_menu::InventoryRegion::Drawer
					})
			})
			.count()
			.max(menu.item_count),
		character_ui_menu::InventoryRegion::Equipped => items
			.iter()
			.filter(|(item, _, slot, _)| {
				item.menu == impulse.entity
					&& slot.is_some_and(|slot| slot.section == session.section)
			})
			.map(|(_, _, slot, _)| slot.map_or(0, |slot| slot.capacity))
			.max()
			.unwrap_or(menu.item_count),
	};
	if count == 0 {
		return;
	}
	let current = menu.selected.min(count - 1);
	let next = match (area.0, session.drawer, nav) {
		(
			character_ui_menu::InventoryRegion::Drawer,
			character_ui_menu::InventoryDrawerMode::Rotary,
			MenuNav::Left,
		) => (current + count - 1) % count,
		(
			character_ui_menu::InventoryRegion::Drawer,
			character_ui_menu::InventoryDrawerMode::Rotary,
			MenuNav::Right,
		) => (current + 1) % count,
		(
			character_ui_menu::InventoryRegion::Drawer,
			character_ui_menu::InventoryDrawerMode::Rotary,
			_,
		) => return,
		(
			character_ui_menu::InventoryRegion::Drawer,
			character_ui_menu::InventoryDrawerMode::Gallery,
			MenuNav::Left,
		) => (current + count - 1) % count,
		(
			character_ui_menu::InventoryRegion::Drawer,
			character_ui_menu::InventoryDrawerMode::Gallery,
			MenuNav::Right,
		) => (current + 1) % count,
		(
			character_ui_menu::InventoryRegion::Drawer,
			character_ui_menu::InventoryDrawerMode::Gallery,
			MenuNav::Up,
		) => {
			let columns = crate::inventory::INVENTORY_GALLERY_COLUMNS;
			let rows = count.div_ceil(columns);
			let row = current / columns;
			let column = current % columns;
			let next_row = (row + rows - 1) % rows;
			(next_row * columns + column).min(count - 1)
		}
		(
			character_ui_menu::InventoryRegion::Drawer,
			character_ui_menu::InventoryDrawerMode::Gallery,
			MenuNav::Down,
		) => {
			let columns = crate::inventory::INVENTORY_GALLERY_COLUMNS;
			let rows = count.div_ceil(columns);
			let row = current / columns;
			let column = current % columns;
			let next_row = (row + 1) % rows;
			(next_row * columns + column).min(count - 1)
		}
		(character_ui_menu::InventoryRegion::Equipped, _, MenuNav::Up) => {
			(current + count - 1) % count
		}
		(character_ui_menu::InventoryRegion::Equipped, _, MenuNav::Down) => (current + 1) % count,
		_ => return,
	};
	menu.selected = next;
	let key = items.iter().find_map(|(item, _, _, key)| {
		(item.menu == impulse.entity && item.index == next)
			.then(|| key.copied())
			.flatten()
	});
	let drawer_mode = session.drawer;
	let memory = session.active_memory_mut();
	if key.is_some() {
		memory.focused_item = key;
	}
	if area.0 == character_ui_menu::InventoryRegion::Drawer
		&& drawer_mode == character_ui_menu::InventoryDrawerMode::Gallery
	{
		let rows = count.div_ceil(crate::inventory::INVENTORY_GALLERY_COLUMNS);
		let selected_row = next / crate::inventory::INVENTORY_GALLERY_COLUMNS;
		let visible_rows = crate::inventory::INVENTORY_GALLERY_VISIBLE_ROWS;
		let max_scroll = rows.saturating_sub(visible_rows);
		if selected_row < memory.gallery_scroll {
			memory.gallery_scroll = selected_row;
		} else if selected_row >= memory.gallery_scroll + visible_rows {
			memory.gallery_scroll = (selected_row + 1 - visible_rows).min(max_scroll);
		}
		commands
			.entity(session_entity)
			.insert(InventoryKeyboardScrollIntent(memory.gallery_scroll));
	}
	if let Some(key) = key {
		session.focus.0 = Some(key);
	}
}

/// Keep section memory aligned with pointer focus and host-driven menu selection.
pub(crate) fn sync_inventory_focus_from_menu(
	menus: Query<(
		Entity,
		&HudMenu,
		&crate::inventory::InventoryScreenArea,
		Option<&HudOverlayMenu>,
	)>,
	parents: Query<&ChildOf>,
	items: Query<(
		&HudMenuItem,
		Option<&crate::inventory::InventoryTile>,
		Option<&InventoryItemKey>,
	)>,
	children: Query<&Children>,
	scrolls: Query<
		&ScrollPosition,
		(With<menu_components::HudScrollViewport>, Changed<ScrollPosition>),
	>,
	keyboard_scroll: Query<&InventoryKeyboardScrollIntent>,
	mut sessions: Query<&mut InventoryMenuSession>,
) {
	for (menu_entity, menu, area, overlay) in &menus {
		if overlay.is_none() {
			continue;
		}
		let session_entity = std::iter::once(menu_entity)
			.chain(parents.iter_ancestors(menu_entity))
			.find(|entity| sessions.contains(*entity));
		let Some(session_entity) = session_entity else { continue };
		let Ok(mut session) = sessions.get_mut(session_entity) else { continue };
		let keyboard_row_pending = keyboard_scroll.contains(session_entity);
		if session.region != area.0 {
			continue;
		}
		let page_is_current = items.iter().any(|(item, tile, _)| {
			item.menu == menu_entity && tile.is_some_and(|tile| tile.section == session.section)
		});
		if !page_is_current {
			continue;
		}
		let key = items.iter().find_map(|(item, tile, key)| {
			(item.menu == menu_entity
				&& item.index == menu.selected
				&& tile.is_none_or(|tile| tile.section == session.section))
			.then(|| key.copied())
			.flatten()
		});
		if let Some(key) = key {
			if session.active_memory().focused_item != Some(key) {
				session.active_memory_mut().focused_item = Some(key);
			}
			if session.focus.0 != Some(key) {
				session.focus.0 = Some(key);
			}
		}
		if !keyboard_row_pending
			&& area.0 == character_ui_menu::InventoryRegion::Drawer
			&& session.drawer == character_ui_menu::InventoryDrawerMode::Gallery
		{
			if let Some(scroll_entity) = std::iter::once(menu_entity)
				.chain(children.iter_descendants(menu_entity))
				.find(|entity| scrolls.contains(*entity))
			{
				if let Ok(scroll) = scrolls.get(scroll_entity) {
					let row = (scroll.y / crate::inventory::INVENTORY_GALLERY_ROW_STEP)
						.floor()
						.max(0.0) as usize;
					if session.active_memory().gallery_scroll != row {
						session.active_memory_mut().gallery_scroll = row;
					}
				}
			}
		}
	}
}

/// Update active-region capture and exact selection without rebuilding the drawer.
pub(crate) fn sync_inventory_region_focus(
	viewports: Query<
		(
			Entity,
			&InventoryMenuSession,
			Option<&InventoryPresentationSnapshot>,
			Option<&InventoryKeyboardScrollIntent>,
		),
		(
			With<OverlaySelectViewport>,
			Or<(Changed<InventoryMenuSession>, With<InventoryKeyboardScrollIntent>)>,
		),
	>,
	children: Query<&Children>,
	areas: Query<(Entity, &crate::inventory::InventoryScreenArea, Option<&HudOverlayMenu>)>,
	items: Query<(&HudMenuItem, &InventoryItemKey)>,
	mut menus: Query<&mut HudMenu>,
	mut scrolls: Query<
		(&mut ScrollPosition, &ComputedNode),
		With<menu_components::HudScrollViewport>,
	>,
	mut commands: Commands,
) {
	for (viewport, session, previous, keyboard_scroll) in &viewports {
		let current = InventoryPresentationSnapshot::from_session(session);
		let presentation_changed = previous.is_none_or(|previous| *previous != current);
		let focus = session.active_memory().focused_item;
		let mut gallery_found = false;
		let mut keyboard_scroll_applied = false;
		for entity in std::iter::once(viewport).chain(children.iter_descendants(viewport)) {
			let Ok((menu_entity, area, overlay)) = areas.get(entity) else { continue };
			let active = area.0 == session.region;
			match (active, overlay.is_some()) {
				(true, false) => {
					commands.entity(menu_entity).insert(HudOverlayMenu);
				}
				(false, true) => {
					commands.entity(menu_entity).remove::<HudOverlayMenu>();
				}
				_ => {}
			}
			if !active {
				continue;
			}
			if presentation_changed {
				if let Some(focus) = focus {
					if let Some((item, _)) =
						items.iter().find(|(item, key)| item.menu == menu_entity && **key == focus)
					{
						if let Ok(mut menu) = menus.get_mut(menu_entity) {
							menu.selected = item.index;
						}
					}
				}
			}
			if (presentation_changed || keyboard_scroll.is_some())
				&& area.0 == character_ui_menu::InventoryRegion::Drawer
				&& session.drawer == character_ui_menu::InventoryDrawerMode::Gallery
			{
				gallery_found = true;
				if let Some(scroll_entity) = std::iter::once(menu_entity)
					.chain(children.iter_descendants(menu_entity))
					.find(|child| scrolls.contains(*child))
				{
					if let Ok((mut scroll, computed)) = scrolls.get_mut(scroll_entity) {
						if computed.size().y <= 0.0 || computed.content_size().y <= 0.0 {
							continue;
						}
						let pointer_or_host_scrolled = scroll.is_changed() && !scroll.is_added();
						let scale = computed.inverse_scale_factor();
						let max_scroll =
							((computed.content_size().y - computed.size().y) * scale).max(0.0);
						let row = keyboard_scroll
							.map(|intent| intent.0)
							.unwrap_or_else(|| session.active_memory().gallery_scroll);
						if !pointer_or_host_scrolled || keyboard_scroll.is_some() {
							scroll.y = (row as f32 * crate::inventory::INVENTORY_GALLERY_ROW_STEP)
								.min(max_scroll);
							keyboard_scroll_applied = keyboard_scroll.is_some();
						}
					}
				}
			}
		}
		if presentation_changed {
			commands.entity(viewport).insert(current);
		}
		if keyboard_scroll.is_some() && (keyboard_scroll_applied || !gallery_found) {
			commands.entity(viewport).remove::<InventoryKeyboardScrollIntent>();
		}
	}
}

pub fn emit_hud_activate_on_click<E: Copy + Send + Sync + 'static>(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	buttons: Query<&MenuButton<E>>,
	keys: Query<&InventoryItemKey>,
	items: Query<&HudMenuItem>,
	mut menus: Query<&mut HudMenu>,
	parents: Query<&ChildOf>,
	mut sessions: Query<&mut InventoryMenuSession>,
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
		if let Ok(mut menu) = menus.get_mut(menu_entity) {
			if let Ok(item) = items.get(click.entity) {
				menu.selected = item.index;
			}
			drop(menu);
			let session_entity = std::iter::once(menu_entity)
				.chain(parents.iter_ancestors(menu_entity))
				.find(|entity| sessions.contains(*entity));
			if let Some(session_entity) = session_entity {
				if let Ok(mut session) = sessions.get_mut(session_entity) {
					if activate_inventory_item(
						click.entity,
						menu_entity,
						&keys,
						&mut session,
						&mut commands,
					) {
						return;
					}
					commands.trigger(MenuActivate { entity: click.entity, choice: button.0 });
					return;
				}
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

fn activate_inventory_item(
	entity: Entity,
	menu: Entity,
	keys: &Query<&InventoryItemKey>,
	session: &mut InventoryMenuSession,
	commands: &mut Commands,
) -> bool {
	let Ok(key) = keys.get(entity) else {
		if session.interaction.reordering().is_some() {
			session.interaction = InventoryInteraction::Browse;
			commands.entity(menu).try_remove::<HudMenuNavLocked>();
		}
		return false;
	};
	if key.equipped && key.region == character_ui_menu::InventoryRegion::Drawer {
		let target =
			InventoryItemKey { region: character_ui_menu::InventoryRegion::Equipped, ..*key };
		session.region = character_ui_menu::InventoryRegion::Equipped;
		session.active_memory_mut().focused_item = Some(target);
		session.focus.0 = Some(target);
		session.interaction = InventoryInteraction::Browse;
		commands.entity(menu).try_remove::<HudMenuNavLocked>();
		return true;
	}
	if key.equipped && key.region == character_ui_menu::InventoryRegion::Equipped {
		session.region = character_ui_menu::InventoryRegion::Equipped;
		session.active_memory_mut().focused_item = Some(*key);
		session.focus.0 = Some(*key);
		if session.interaction.reordering() == Some(*key) {
			session.interaction = InventoryInteraction::Browse;
			commands.entity(menu).try_remove::<HudMenuNavLocked>();
		} else {
			session.interaction = InventoryInteraction::Reorder { item: *key };
			commands.entity(menu).try_insert(HudMenuNavLocked);
		}
		return true;
	}
	session.region = key.region;
	session.active_memory_mut().focused_item = Some(*key);
	session.focus.0 = Some(*key);
	if session.interaction.reordering().is_some() {
		session.interaction = InventoryInteraction::Browse;
		commands.entity(menu).try_remove::<HudMenuNavLocked>();
	}
	false
}

pub fn emit_overlay_open_on_click(
	click: On<Pointer<Click>>,
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
	click: On<Pointer<Click>>,
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
	parents: Query<&ChildOf>,
	mut sessions: Query<&mut InventoryMenuSession>,
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
	let session_entity = std::iter::once(menu_entity)
		.chain(parents.iter_ancestors(menu_entity))
		.find(|entity| sessions.contains(*entity));
	if let Some(session_entity) = session_entity {
		if let Ok(mut session) = sessions.get_mut(session_entity) {
			if let Some((entity, _, _, _, _)) = items
				.iter()
				.find(|(_, item, _, _, _)| item.menu == menu_entity && item.index == menu.selected)
			{
				if activate_inventory_item(entity, menu_entity, &keys, &mut session, &mut commands)
				{
					return;
				}
			}
			let InventoryMenuSession { interaction, focus, .. } = &mut *session;
			activate_selected_hud_item(
				None,
				menu_entity,
				menu,
				&items,
				&keys,
				interaction,
				focus,
				&mut commands,
			);
			return;
		}
	}
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
	menus: Query<&HudMenu>,
	parents: Query<&ChildOf>,
	mut sessions: Query<&mut InventoryMenuSession>,
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
	let Ok(menu) = menus.get(impulse.entity) else {
		return;
	};
	let session_entity = std::iter::once(impulse.entity)
		.chain(parents.iter_ancestors(impulse.entity))
		.find(|entity| sessions.contains(*entity));
	if let Some(session_entity) = session_entity {
		if let Ok(mut state) = sessions.get_mut(session_entity) {
			if let Some((entity, _, _, _, _)) = items.iter().find(|(_, item, _, _, _)| {
				item.menu == impulse.entity && item.index == menu.selected
			}) {
				if activate_inventory_item(entity, impulse.entity, &keys, &mut state, &mut commands)
				{
					return;
				}
			}
			let InventoryMenuSession { interaction, focus, .. } = &mut *state;
			activate_selected_hud_item(
				Some(impulse.event()),
				impulse.entity,
				menu,
				&items,
				&keys,
				interaction,
				focus,
				&mut commands,
			);
			return;
		}
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
	if key.region == character_ui_menu::InventoryRegion::Drawer {
		let target =
			InventoryItemKey { region: character_ui_menu::InventoryRegion::Equipped, ..*key };
		focus.0 = Some(target);
		exit_reorder(interaction, menu, commands);
		return true;
	}
	focus.0 = Some(*key);
	let same = matches!(*interaction, InventoryInteraction::Reorder { item } if item == *key);
	if same {
		exit_reorder(interaction, menu, commands);
		return true;
	}
	*interaction = InventoryInteraction::Reorder { item: *key };
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
	menus: Query<(), With<HudMenuNavLocked>>,
	sessions: Query<&InventoryMenuSession>,
	parents: Query<&ChildOf>,
	items: Query<(Entity, &HudMenuItem, &InventoryItemKey, &EquippedOrderControls<E>)>,
	mut commands: Commands,
) {
	if menus.get(impulse.entity).is_err() {
		return;
	}
	let state = std::iter::once(impulse.entity)
		.chain(parents.iter_ancestors(impulse.entity))
		.find_map(|entity| sessions.get(entity).ok().copied());
	let Some(item_key) = state.map_or(*interaction, |state| state.interaction).reordering() else {
		return;
	};
	// Hover/focus is positional and can change while the host rebuilds the
	// tiles. Reorder always belongs to the bag item that entered this mode.
	let Some((entity, _, _, controls)) = items
		.iter()
		.find(|(_, item, key, _)| item.menu == impulse.entity && **key == item_key)
	else {
		return;
	};
	let event = match impulse.event().nav {
		MenuNav::Left | MenuNav::Up => controls.move_left,
		MenuNav::Right | MenuNav::Down => controls.move_right,
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
	} else if keyboard.just_pressed(KeyCode::ArrowUp) {
		MenuNav::Up
	} else if keyboard.just_pressed(KeyCode::ArrowDown) {
		MenuNav::Down
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

pub fn consume_inventory_session_keyboard_back(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	stack: Res<menu_components::MenuModalStack>,
	roots: Query<(Entity, &menu_components::MenuModal)>,
	parents: Query<&ChildOf>,
	menus: Query<
		Entity,
		(
			With<HudMenu>,
			With<HudOverlayMenu>,
			With<menu_components::InventoryHudMenu>,
			Without<ScopedMenuSession>,
		),
	>,
	mut sessions: Query<&mut InventoryMenuSession>,
	mut hold: ResMut<MenuModalBackHold>,
	mut nav: ResMut<MenuNavPad>,
	mut commands: Commands,
) {
	if !keyboard_nav.is_enabled()
		|| lock.0
		|| !keyboard.just_pressed(KeyCode::Escape)
		|| inventory_modal_blocks_shell(&stack)
	{
		return;
	}
	for menu in &menus {
		if !menu_components::entity_under_nav_owner(menu, &stack, &roots, &parents) {
			continue;
		}
		let session_entity = std::iter::once(menu)
			.chain(parents.iter_ancestors(menu))
			.find(|entity| sessions.contains(*entity));
		let Some(session_entity) = session_entity else { continue };
		if let Ok(mut session) = sessions.get_mut(session_entity) {
			if session.interaction.reordering().is_none() {
				continue;
			}
			session.interaction = InventoryInteraction::Browse;
			hold.0 = true;
			nav.events.retain(|event| *event != MenuNav::Back);
			commands.entity(menu).try_remove::<HudMenuNavLocked>();
			return;
		}
	}
}

pub fn emit_inventory_unequip_on_action<E: Copy + Send + Sync + 'static>(
	impulse: On<MenuActionImpulse>,
	overlay_menus: Query<(Entity, &HudMenu), With<HudOverlayMenu>>,
	parents: Query<&ChildOf>,
	mut sessions: Query<&mut InventoryMenuSession>,
	items: Query<(
		Entity,
		&HudMenuItem,
		Option<&InventoryItemKey>,
		Option<&EquippedOrderControls<E>>,
	)>,
	mut interaction: ResMut<InventoryInteraction>,
	scoped: Query<(), With<ScopedMenuSession>>,
	mut commands: Commands,
) {
	if impulse.action != MenuAction::Unequip {
		return;
	}
	let Ok((menu_entity, menu)) = overlay_menus.get(impulse.entity) else {
		return;
	};
	let Some((entity, _, key, Some(controls))) = items
		.iter()
		.find(|(_, item, _, _)| item.menu == menu_entity && item.index == menu.selected)
	else {
		return;
	};
	if key.is_some_and(|key| {
		!key.equipped || key.region != character_ui_menu::InventoryRegion::Equipped
	}) {
		return;
	}
	let Some(event) = controls.unequip else {
		return;
	};
	let session_entity = std::iter::once(menu_entity)
		.chain(parents.iter_ancestors(menu_entity))
		.find(|entity| sessions.contains(*entity));
	if let Some(session_entity) = session_entity {
		if let Ok(mut state) = sessions.get_mut(session_entity) {
			state.interaction = InventoryInteraction::Browse;
			if let Some(key) = key {
				state.focus.0 = Some(*key);
				state.active_memory_mut().focused_item = Some(*key);
			}
		}
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
	parents: Query<&ChildOf>,
	mut commands: Commands,
) {
	for (entity, item, key, mark) in &tiles {
		let session_entity = std::iter::once(item.menu)
			.chain(parents.iter_ancestors(item.menu))
			.find(|entity| sessions.contains(*entity));
		let selected = session_entity
			.and_then(|entity| sessions.get(entity).ok())
			.map(|s| s.interaction)
			.unwrap_or_else(|| {
				if std::iter::once(item.menu)
					.chain(parents.iter_ancestors(item.menu))
					.any(|entity| scoped.contains(entity))
				{
					InventoryInteraction::Browse
				} else {
					*interaction
				}
			})
			.reordering();
		let want = selected == Some(*key) && key.equipped;
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
		use character_ui_menu::InventoryRegion;
		use menu_components::MenuModalBackHold;

		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<ButtonInput<KeyCode>>();
		world.init_resource::<MenuModalBackHold>();
		world.init_resource::<InventoryInteraction>();
		world.init_resource::<Messages<MenuModalDismiss>>();
		*world.resource_mut::<InventoryInteraction>() = InventoryInteraction::Reorder {
			item: crate::InventoryItemKey {
				index: 3,
				equipped: true,
				region: InventoryRegion::Equipped,
			},
		};
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
		use character_ui_menu::InventoryRegion;
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
		*world.resource_mut::<InventoryInteraction>() = InventoryInteraction::Reorder {
			item: crate::InventoryItemKey {
				index: 7,
				equipped: true,
				region: InventoryRegion::Equipped,
			},
		};
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
	parents: Query<&ChildOf>,
	mut hold: ResMut<MenuModalBackHold>,
	mut nav: ResMut<MenuNavPad>,
	mut commands: Commands,
) {
	if impulse.nav != MenuNav::Back {
		return;
	}
	let Some(session_entity) = std::iter::once(impulse.entity)
		.chain(parents.iter_ancestors(impulse.entity))
		.find(|entity| sessions.contains(*entity))
	else {
		return;
	};
	let Ok(mut state) = sessions.get_mut(session_entity) else { return };
	if state.interaction.reordering().is_none() {
		return;
	}
	state.interaction = InventoryInteraction::Browse;
	commands.entity(impulse.entity).try_remove::<HudMenuNavLocked>();
	hold.0 = true;
	nav.events.retain(|event| *event != MenuNav::Back);
	impulse.propagate(false);
}

#[cfg(test)]
mod scoped_tests {
	use crate::{
		EquippedOrderControls, InventoryFocusHint, InventoryInteraction, InventoryItemKey,
		InventoryMenuSession, MenuButton, OverlaySelectViewport,
	};
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use character_ui_menu::InventoryRegion;
	use menu_components::{
		HudMenu, HudMenuItem, HudMenuNavLocked, HudOverlayMenu, InventoryHudMenu, MenuAction,
		MenuActionImpulse, MenuActivate, MenuNav, MenuNavImpulse, ScopedMenuSession,
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
		*world.resource_mut::<InventoryInteraction>() = InventoryInteraction::Reorder {
			item: InventoryItemKey { index: 99, equipped: true, region: InventoryRegion::Equipped },
		};
		let session = || InventoryMenuSession {
			interaction: InventoryInteraction::Reorder {
				item: InventoryItemKey {
					index: 7,
					equipped: true,
					region: InventoryRegion::Equipped,
				},
			},
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
			InventoryItemKey { index: 7, equipped: true, region: InventoryRegion::Equipped },
			controls(10u8),
		));
		let stable = world
			.spawn((
				HudMenuItem { menu: second, index: 1 },
				InventoryItemKey { index: 7, equipped: true, region: InventoryRegion::Equipped },
				controls(20u8),
			))
			.id();
		let selected = world.spawn((HudMenuItem { menu: second, index: 0 }, controls(30u8))).id();
		world.trigger(MenuNavImpulse::new(second, MenuNav::Up));
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
			.is_some_and(|s| s.interaction.reordering().is_some_and(|key| key.index == 7)));
		assert!(world.get::<HudMenuNavLocked>(first).is_some());
		assert!(world.get::<HudMenuNavLocked>(second).is_none());
		assert_eq!(
			world.resource::<InventoryInteraction>().reordering(),
			Some(InventoryItemKey { index: 99, equipped: true, region: InventoryRegion::Equipped })
		);
		Ok(())
	}

	#[test]
	fn viewport_session_controls_child_aiop_select_and_back_without_closing_overlay() {
		use super::consume_scoped_inventory_reorder_back;
		use menu_components::{MenuModalBackHold, MenuModalLayer, MenuModalStack, MenuNavPad};
		let mut world = World::new();
		world.init_resource::<InventoryInteraction>();
		world.init_resource::<InventoryFocusHint>();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<MenuModalBackHold>();
		world.init_resource::<MenuNavPad>();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::OVERLAY_SELECT);
		world.add_observer(super::emit_hud_activate_on_nav::<u8>);
		world.add_observer(consume_scoped_inventory_reorder_back);
		let key = InventoryItemKey { index: 12, equipped: true, region: InventoryRegion::Equipped };
		let viewport = world.spawn(InventoryMenuSession::default()).id();
		let menu = world.spawn((HudMenu::new(1), HudOverlayMenu)).id();
		world.entity_mut(viewport).add_child(menu);
		world.spawn((HudMenuItem { menu, index: 0 }, key, MenuButton(7u8)));
		world.trigger(MenuNavImpulse::new(menu, MenuNav::Select));
		world.flush();
		assert_eq!(
			world.get::<InventoryMenuSession>(viewport).map(|s| s.interaction),
			Some(InventoryInteraction::Reorder { item: key })
		);
		assert!(world.get::<HudMenuNavLocked>(menu).is_some());
		world.resource_mut::<MenuNavPad>().events.push(MenuNav::Back);
		world.trigger(MenuNavImpulse::new(menu, MenuNav::Back));
		world.flush();
		assert_eq!(
			world.get::<InventoryMenuSession>(viewport).map(|s| s.interaction),
			Some(InventoryInteraction::Browse)
		);
		assert!(world.get::<HudMenuNavLocked>(menu).is_none());
		assert!(world.resource::<MenuModalBackHold>().0);
		assert!(!world.resource::<MenuNavPad>().just_pressed(MenuNav::Back));
	}

	#[derive(Resource, Default)]
	struct Activations(usize);
	fn count_activation(_event: On<MenuActivate<u8>>, mut hits: ResMut<Activations>) {
		hits.0 += 1;
	}

	#[test]
	fn checked_drawer_select_moves_focus_to_aiop_representation() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<InventoryInteraction>();
		world.init_resource::<InventoryFocusHint>();
		world.init_resource::<Activations>();
		world.add_observer(super::emit_hud_activate_on_nav::<u8>);
		world.add_observer(count_activation);
		let viewport = world.spawn(InventoryMenuSession::default()).id();
		let menu = world.spawn(HudMenu::new(1)).id();
		world.entity_mut(viewport).add_child(menu);
		let drawer_key =
			InventoryItemKey { index: 4, equipped: true, region: InventoryRegion::Drawer };
		world.spawn((HudMenuItem { menu, index: 0 }, drawer_key, MenuButton(9u8)));
		world.trigger(MenuNavImpulse::new(menu, MenuNav::Select));
		world.flush();
		let session = world
			.get::<InventoryMenuSession>(viewport)
			.ok_or_else(|| anyhow::anyhow!("viewport state"))?;
		assert_eq!(session.region, InventoryRegion::Equipped);
		assert_eq!(
			session.active_memory().focused_item,
			Some(InventoryItemKey { region: InventoryRegion::Equipped, ..drawer_key })
		);
		assert_eq!(session.interaction, InventoryInteraction::Browse);
		assert_eq!(world.resource::<Activations>().0, 0);
		Ok(())
	}

	#[test]
	fn region_toggle_updates_overlay_owner_and_follows_identity_without_rebuild(
	) -> anyhow::Result<()> {
		use super::sync_inventory_region_focus;
		use crate::inventory::InventoryScreenArea;
		let mut world = World::new();
		let drawer_key =
			InventoryItemKey { index: 5, equipped: true, region: InventoryRegion::Drawer };
		let equipped_key = InventoryItemKey { region: InventoryRegion::Equipped, ..drawer_key };
		let mut session = InventoryMenuSession::default();
		session.section_memory[session.section.index()][InventoryRegion::Drawer.index()]
			.focused_item = Some(drawer_key);
		session.section_memory[session.section.index()][InventoryRegion::Equipped.index()]
			.focused_item = Some(equipped_key);
		let viewport = world.spawn((OverlaySelectViewport, session)).id();
		let drawer = world
			.spawn((
				InventoryHudMenu,
				InventoryScreenArea(InventoryRegion::Drawer),
				HudMenu::new(2),
				HudOverlayMenu,
			))
			.id();
		let equipped = world
			.spawn((
				InventoryHudMenu,
				InventoryScreenArea(InventoryRegion::Equipped),
				HudMenu::new(3),
			))
			.id();
		world.entity_mut(viewport).add_children(&[drawer, equipped]);
		world.spawn((HudMenuItem { menu: drawer, index: 1 }, drawer_key));
		world.spawn((HudMenuItem { menu: equipped, index: 2 }, equipped_key));
		world
			.run_system_once(sync_inventory_region_focus)
			.map_err(|error| anyhow::anyhow!("initial region sync: {error:?}"))?;
		world.flush();
		assert!(world.get::<HudOverlayMenu>(drawer).is_some());
		assert!(world.get::<HudOverlayMenu>(equipped).is_none());
		world
			.get_mut::<InventoryMenuSession>(viewport)
			.ok_or_else(|| anyhow::anyhow!("session"))?
			.region = InventoryRegion::Equipped;
		world
			.run_system_once(sync_inventory_region_focus)
			.map_err(|error| anyhow::anyhow!("switched region sync: {error:?}"))?;
		world.flush();
		assert!(world.get::<HudOverlayMenu>(drawer).is_none());
		assert!(world.get::<HudOverlayMenu>(equipped).is_some());
		assert_eq!(world.get::<HudMenu>(equipped).map(|menu| menu.selected), Some(2));
		Ok(())
	}

	#[test]
	fn page_and_drawer_actions_cycle_restore_memory_and_pause_during_reorder() {
		let mut session = InventoryMenuSession::default();
		let drawer_item =
			InventoryItemKey { index: 2, equipped: false, region: InventoryRegion::Drawer };
		session.active_memory_mut().focused_item = Some(drawer_item);
		super::apply_inventory_navigation(
			&mut session,
			menu_components::InventoryNavigation::PreviousSection,
		);
		assert_eq!(session.section, character_ui_menu::InventorySection::Lubikan);
		super::apply_inventory_navigation(
			&mut session,
			menu_components::InventoryNavigation::ToggleDrawer,
		);
		assert_eq!(session.drawer, character_ui_menu::InventoryDrawerMode::Gallery);
		super::apply_inventory_navigation(
			&mut session,
			menu_components::InventoryNavigation::ToggleRegion,
		);
		assert_eq!(session.region, InventoryRegion::Equipped);
		super::apply_inventory_navigation(
			&mut session,
			menu_components::InventoryNavigation::FocusDrawer,
		);
		assert_eq!(session.region, InventoryRegion::Drawer);
		super::apply_inventory_navigation(
			&mut session,
			menu_components::InventoryNavigation::FocusDrawer,
		);
		assert_eq!(session.region, InventoryRegion::Drawer);
		let before = session;
		session.interaction = InventoryInteraction::Reorder {
			item: InventoryItemKey { index: 9, equipped: true, region: InventoryRegion::Equipped },
		};
		super::apply_inventory_navigation(
			&mut session,
			menu_components::InventoryNavigation::NextSection,
		);
		assert_eq!(session.section, before.section);
	}

	#[test]
	fn page_change_does_not_capture_focus_from_stale_menu_children() -> anyhow::Result<()> {
		use super::sync_inventory_focus_from_menu;
		use crate::inventory::{InventoryScreenArea, InventoryTile};
		use character_ui_menu::InventorySection;
		let mut world = World::new();
		let remembered =
			InventoryItemKey { index: 20, equipped: false, region: InventoryRegion::Drawer };
		let mut session = InventoryMenuSession::default();
		session.section_memory[InventorySection::Tools.index()][InventoryRegion::Drawer.index()]
			.focused_item = Some(remembered);
		session.section = InventorySection::Tools;
		session.section_memory[InventorySection::Tools.index()][InventoryRegion::Drawer.index()]
			.gallery_scroll = 2;
		let viewport = world.spawn((OverlaySelectViewport, session)).id();
		let menu = world
			.spawn((
				InventoryHudMenu,
				InventoryScreenArea(InventoryRegion::Drawer),
				HudOverlayMenu,
				HudMenu::new(1),
			))
			.id();
		world.entity_mut(viewport).add_child(menu);
		let old_page = world
			.spawn((
				HudMenuItem { menu, index: 0 },
				InventoryTile {
					section: InventorySection::Clothing,
					region: InventoryRegion::Drawer,
					ordinal: 0,
				},
				InventoryItemKey { index: 1, equipped: false, region: InventoryRegion::Drawer },
			))
			.id();
		world
			.run_system_once(sync_inventory_focus_from_menu)
			.map_err(|error| anyhow::anyhow!("stale page focus capture: {error:?}"))?;
		assert_eq!(
			world
				.get::<InventoryMenuSession>(viewport)
				.and_then(|session| session.section_memory[InventorySection::Tools.index()]
					[InventoryRegion::Drawer.index()]
				.focused_item),
			Some(remembered)
		);
		world.despawn(old_page);
		let current_page_key =
			InventoryItemKey { index: 21, equipped: false, region: InventoryRegion::Drawer };
		world.spawn((
			HudMenuItem { menu, index: 0 },
			InventoryTile {
				section: InventorySection::Tools,
				region: InventoryRegion::Drawer,
				ordinal: 0,
			},
			current_page_key,
		));
		world
			.run_system_once(sync_inventory_focus_from_menu)
			.map_err(|error| anyhow::anyhow!("current page focus capture: {error:?}"))?;
		assert_eq!(
			world
				.get::<InventoryMenuSession>(viewport)
				.and_then(|session| session.section_memory[InventorySection::Tools.index()]
					[InventoryRegion::Drawer.index()]
				.focused_item),
			Some(current_page_key)
		);
		Ok(())
	}

	fn gallery_scroll_fixture() -> anyhow::Result<(App, Entity, Entity, Vec<Entity>)> {
		use crate::inventory::{InventoryScreenArea, InventoryTile};
		use character_ui_menu::{InventoryDrawerMode, InventoryRegion, InventorySection};
		use menu_components::{HudScrollManual, HudScrollViewport, MenuModalStack};
		let mut app = App::new();
		app.init_resource::<MenuModalStack>();
		app.add_observer(super::navigate_inventory_menu);
		app.add_systems(
			Update,
			(super::sync_inventory_focus_from_menu, super::sync_inventory_region_focus).chain(),
		);
		let focused =
			InventoryItemKey { index: 4, equipped: false, region: InventoryRegion::Drawer };
		let mut session = InventoryMenuSession::default();
		session.drawer = InventoryDrawerMode::Gallery;
		session.active_memory_mut().focused_item = Some(focused);
		let viewport = app.world_mut().spawn((OverlaySelectViewport, session)).id();
		let menu = app
			.world_mut()
			.spawn((
				InventoryHudMenu,
				InventoryScreenArea(InventoryRegion::Drawer),
				HudOverlayMenu,
				HudMenu { selected: 4, item_count: 16 },
				HudScrollManual,
				HudScrollViewport,
				ScrollPosition::default(),
				UiGlobalTransform::from_xy(0.0, 0.0),
				ComputedNode {
					size: Vec2::new(400.0, 200.0),
					content_size: Vec2::new(400.0, 400.0),
					..default()
				},
			))
			.id();
		app.world_mut().entity_mut(viewport).add_child(menu);
		let mut tiles = Vec::new();
		for index in 0..16 {
			let key = InventoryItemKey { index, equipped: false, region: InventoryRegion::Drawer };
			let tile = app
				.world_mut()
				.spawn((
					HudMenuItem { menu, index },
					InventoryTile {
						section: InventorySection::Clothing,
						region: InventoryRegion::Drawer,
						ordinal: index,
					},
					key,
				))
				.id();
			app.world_mut().entity_mut(menu).add_child(tile);
			tiles.push(tile);
		}
		app.update();
		Ok((app, viewport, menu, tiles))
	}

	#[test]
	fn gallery_keyboard_pointer_scroll_arbitration_keeps_focus_visible() -> anyhow::Result<()> {
		use character_ui_menu::{InventoryRegion, InventorySection};
		use menu_components::{HudScrollManual, HudScrollViewport};
		let (mut app, viewport, menu, _) = gallery_scroll_fixture()?;
		app.world_mut().trigger(MenuNavImpulse::new(menu, MenuNav::Down));
		app.world_mut().flush();
		app.update();
		let session = app
			.world()
			.get::<InventoryMenuSession>(viewport)
			.ok_or_else(|| anyhow::anyhow!("inventory session"))?;
		assert_eq!(app.world().get::<HudMenu>(menu).map(|menu| menu.selected), Some(9));
		assert_eq!(session.active_memory().gallery_scroll, 0);
		assert_eq!(app.world().get::<ScrollPosition>(menu).map(|scroll| scroll.y), Some(0.0));
		app.world_mut().trigger(MenuNavImpulse::new(menu, MenuNav::Down));
		app.world_mut().flush();
		app.update();
		let session = app
			.world()
			.get::<InventoryMenuSession>(viewport)
			.ok_or_else(|| anyhow::anyhow!("inventory session"))?;
		assert_eq!(app.world().get::<HudMenu>(menu).map(|menu| menu.selected), Some(14));
		assert_eq!(session.active_memory().gallery_scroll, 1);
		assert_eq!(
			app.world().get::<ScrollPosition>(menu).map(|scroll| scroll.y),
			Some(crate::inventory::INVENTORY_GALLERY_ROW_STEP)
		);
		app.world_mut().trigger(MenuNavImpulse::new(menu, MenuNav::Down));
		app.world_mut().flush();
		app.update();
		assert_eq!(app.world().get::<HudMenu>(menu).map(|menu| menu.selected), Some(15));
		assert_eq!(
			app.world()
				.get::<InventoryMenuSession>(viewport)
				.ok_or_else(|| anyhow::anyhow!("inventory session"))?
				.active_memory()
				.gallery_scroll,
			2
		);

		// A pointer-only wheel offset is retained exactly and remembered by row.
		app.world_mut()
			.get_mut::<ScrollPosition>(menu)
			.ok_or_else(|| anyhow::anyhow!("gallery scroll position"))?
			.y = 140.0;
		app.update();
		assert_eq!(app.world().get::<ScrollPosition>(menu).map(|scroll| scroll.y), Some(140.0));
		assert_eq!(
			app.world()
				.get::<InventoryMenuSession>(viewport)
				.ok_or_else(|| anyhow::anyhow!("inventory session"))?
				.active_memory()
				.gallery_scroll,
			1
		);

		// A keyboard move in the same frame as a new pointer offset wins only
		// when its focused row would otherwise be outside the visible window.
		let item_nine =
			InventoryItemKey { index: 9, equipped: false, region: InventoryRegion::Drawer };
		{
			let mut session = app
				.world_mut()
				.get_mut::<InventoryMenuSession>(viewport)
				.ok_or_else(|| anyhow::anyhow!("inventory session"))?;
			session.active_memory_mut().focused_item = Some(item_nine);
			session.active_memory_mut().gallery_scroll = 0;
			session.focus.0 = Some(item_nine);
		}
		app.world_mut()
			.get_mut::<HudMenu>(menu)
			.ok_or_else(|| anyhow::anyhow!("gallery menu"))?
			.selected = 9;
		app.update();
		app.world_mut()
			.get_mut::<ScrollPosition>(menu)
			.ok_or_else(|| anyhow::anyhow!("gallery scroll"))?
			.y = 10.0;
		app.world_mut().trigger(MenuNavImpulse::new(menu, MenuNav::Down));
		app.world_mut().flush();
		app.update();
		let session = app
			.world()
			.get::<InventoryMenuSession>(viewport)
			.ok_or_else(|| anyhow::anyhow!("inventory session"))?;
		assert_eq!(app.world().get::<HudMenu>(menu).map(|menu| menu.selected), Some(14));
		assert_eq!(session.active_memory().gallery_scroll, 1);
		assert_eq!(
			app.world().get::<ScrollPosition>(menu).map(|scroll| scroll.y),
			Some(crate::inventory::INVENTORY_GALLERY_ROW_STEP)
		);
		assert_eq!(session.section, InventorySection::Clothing);
		assert!(app.world().get::<HudScrollManual>(menu).is_some());
		assert!(app.world().get::<HudScrollViewport>(menu).is_some());

		// Even movement within the same saved two-row window reveals the
		// keyboard-focused row after a fractional pointer offset.
		let item_five =
			InventoryItemKey { index: 5, equipped: false, region: InventoryRegion::Drawer };
		{
			let mut session = app
				.world_mut()
				.get_mut::<InventoryMenuSession>(viewport)
				.ok_or_else(|| anyhow::anyhow!("inventory session"))?;
			session.active_memory_mut().focused_item = Some(item_five);
			session.active_memory_mut().gallery_scroll = 0;
			session.focus.0 = Some(item_five);
		}
		app.world_mut()
			.get_mut::<HudMenu>(menu)
			.ok_or_else(|| anyhow::anyhow!("gallery menu"))?
			.selected = 5;
		app.update();
		app.world_mut()
			.get_mut::<ScrollPosition>(menu)
			.ok_or_else(|| anyhow::anyhow!("gallery scroll"))?
			.y = 20.0;
		app.world_mut().trigger(MenuNavImpulse::new(menu, MenuNav::Up));
		app.world_mut().flush();
		app.update();
		assert_eq!(app.world().get::<HudMenu>(menu).map(|menu| menu.selected), Some(0));
		assert_eq!(app.world().get::<ScrollPosition>(menu).map(|scroll| scroll.y), Some(0.0));
		Ok(())
	}

	#[test]
	fn manual_gallery_scroll_does_not_disable_equipped_item_reveal() -> anyhow::Result<()> {
		use crate::inventory::InventoryScreenArea;
		use character_ui_menu::InventoryRegion;
		use menu_components::{scroll_hud_selection_into_view, HudScrollManual, HudScrollViewport};
		let (mut app, viewport, gallery, tiles) = gallery_scroll_fixture()?;
		let focus_tile = *tiles.get(14).ok_or_else(|| anyhow::anyhow!("gallery item 14"))?;
		app.world_mut().entity_mut(focus_tile).insert((
			ComputedNode { size: Vec2::new(80.0, 80.0), ..default() },
			UiGlobalTransform::from_xy(0.0, 300.0),
		));
		app.world_mut()
			.get_mut::<HudMenu>(gallery)
			.ok_or_else(|| anyhow::anyhow!("gallery menu"))?
			.selected = 14;
		app.world_mut()
			.get_mut::<ScrollPosition>(gallery)
			.ok_or_else(|| anyhow::anyhow!("gallery scroll"))?
			.y = 40.0;
		let equipped = app
			.world_mut()
			.spawn((
				InventoryScreenArea(InventoryRegion::Equipped),
				HudMenu::new(1),
				HudScrollViewport,
				ScrollPosition::default(),
				UiGlobalTransform::from_xy(0.0, 0.0),
				ComputedNode {
					size: Vec2::new(400.0, 200.0),
					content_size: Vec2::new(400.0, 500.0),
					..default()
				},
			))
			.id();
		app.world_mut().entity_mut(viewport).add_child(equipped);
		let equipped_tile = app
			.world_mut()
			.spawn((
				HudMenuItem { menu: equipped, index: 0 },
				ComputedNode { size: Vec2::new(80.0, 80.0), ..default() },
				UiGlobalTransform::from_xy(0.0, 300.0),
			))
			.id();
		app.world_mut().entity_mut(equipped).add_child(equipped_tile);
		app.world_mut()
			.run_system_once(scroll_hud_selection_into_view)
			.map_err(|error| anyhow::anyhow!("selection scroll system: {error:?}"))?;
		assert_eq!(app.world().get::<ScrollPosition>(gallery).map(|scroll| scroll.y), Some(40.0));
		assert!(
			app.world().get::<ScrollPosition>(equipped).is_some_and(|scroll| scroll.y > 0.0),
			"equipped selection should still reveal its focused slot"
		);
		assert!(app.world().get::<HudScrollManual>(gallery).is_some());
		Ok(())
	}

	#[test]
	fn gallery_saved_scroll_survives_unlaid_out_viewport_until_keyboard_reveal_is_ready(
	) -> anyhow::Result<()> {
		use crate::inventory::InventoryScreenArea;
		use character_ui_menu::{InventoryDrawerMode, InventoryRegion};
		use menu_components::{HudScrollManual, HudScrollViewport};
		let mut world = World::new();
		let mut session = InventoryMenuSession::default();
		session.drawer = InventoryDrawerMode::Gallery;
		session.active_memory_mut().gallery_scroll = 1;
		let viewport = world.spawn((OverlaySelectViewport, session)).id();
		let menu = world
			.spawn((
				InventoryScreenArea(InventoryRegion::Drawer),
				HudScrollManual,
				HudScrollViewport,
				ScrollPosition(Vec2::new(0.0, crate::inventory::INVENTORY_GALLERY_ROW_STEP)),
				ComputedNode::default(),
			))
			.id();
		world.entity_mut(viewport).add_child(menu);
		world
			.run_system_once(super::sync_inventory_region_focus)
			.map_err(|error| anyhow::anyhow!("initial gallery presentation: {error:?}"))?;
		world.flush();
		assert_eq!(
			world.get::<ScrollPosition>(menu).map(|scroll| scroll.y),
			Some(crate::inventory::INVENTORY_GALLERY_ROW_STEP)
		);

		world.entity_mut(viewport).insert(super::InventoryKeyboardScrollIntent(1));
		world
			.get_mut::<ScrollPosition>(menu)
			.ok_or_else(|| anyhow::anyhow!("gallery scroll"))?
			.y = 0.0;
		world
			.run_system_once(super::sync_inventory_region_focus)
			.map_err(|error| anyhow::anyhow!("unlaid-out keyboard reveal: {error:?}"))?;
		world.flush();
		assert!(world.get::<super::InventoryKeyboardScrollIntent>(viewport).is_some());
		assert_eq!(world.get::<ScrollPosition>(menu).map(|scroll| scroll.y), Some(0.0));

		*world
			.get_mut::<ComputedNode>(menu)
			.ok_or_else(|| anyhow::anyhow!("gallery computed node"))? = ComputedNode {
			size: Vec2::new(400.0, 200.0),
			content_size: Vec2::new(400.0, 500.0),
			..default()
		};
		world
			.run_system_once(super::sync_inventory_region_focus)
			.map_err(|error| anyhow::anyhow!("laid-out keyboard reveal: {error:?}"))?;
		world.flush();
		assert!(world.get::<super::InventoryKeyboardScrollIntent>(viewport).is_none());
		assert_eq!(
			world.get::<ScrollPosition>(menu).map(|scroll| scroll.y),
			Some(crate::inventory::INVENTORY_GALLERY_ROW_STEP)
		);
		Ok(())
	}
}
