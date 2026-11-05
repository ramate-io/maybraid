//! Exercise item identity across the real layout → input → inventory → rebuild path.
use anyhow::{Context as ContextExt, Result};
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use character_creation_menus::{CharacterMenu, MenuEvent};
use character_items::{ClothingMaterial, ClothingMesh, InventoryItem, ItemColor};
use character_ui_menu::MenuComponent;
use maybraid_character_ui_menu_renderer::{
	fill_inventory_layout, find_overlay_node,
	input::{
		emit_hud_activate_on_nav, emit_inventory_reorder_keyboard, emit_inventory_reorder_nav,
	},
	InventoryFocusHint, InventoryInteraction, InventoryItemKey, NoThumbnails,
};
use menu_components::{
	apply_hud_menu_nav, navigate_hud_menus, HudFonts, HudMenu, HudMenuItem, HudMenuNavLocked,
	HudOverlayMenu, KeyboardMenuNav, MenuModalStack, TextMenuInputLock,
};
use menu_components::{MenuNav, MenuNavImpulse};

use crate::character::{
	on_menu_activate, CharacterMenuState, CharacterScreen, CharacterUiSyncState,
	OverlaySelectState, OverlayUiSyncState,
};
use maybraid_character_ui_menu_renderer::CharacterMenuEvent;
use menu_components::ActiveOverlayKey;

fn fixture() -> (World, Entity) {
	let items = (0..5)
		.map(|_| {
			InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red)
		})
		.collect();
	let mut menu = CharacterMenu::for_create(items);
	for index in 0..5 {
		menu.apply(MenuEvent::EquipInventory(index));
	}
	let mut world = World::new();
	world.insert_resource(CharacterMenuState(menu));
	world.init_resource::<CharacterUiSyncState>();
	world.init_resource::<OverlaySelectState>();
	world.init_resource::<OverlayUiSyncState>();
	world.init_resource::<ActiveOverlayKey>();
	world.init_resource::<Messages<CharacterMenuEvent<MenuEvent>>>();
	world.init_resource::<InventoryInteraction>();
	world.init_resource::<InventoryFocusHint>();
	world.init_resource::<TextMenuInputLock>();
	world.init_resource::<MenuModalStack>();
	world.init_resource::<KeyboardMenuNav>();
	world.init_resource::<ButtonInput<KeyCode>>();
	world.add_observer(apply_hud_menu_nav);
	world.add_observer(emit_hud_activate_on_nav::<MenuEvent>);
	world.add_observer(emit_inventory_reorder_nav::<MenuEvent>);
	world.add_observer(on_menu_activate);
	let screen = world.spawn(CharacterScreen).id();
	let viewport = world.spawn((HudMenu::new(0), HudOverlayMenu, ChildOf(screen))).id();
	(world, viewport)
}

fn rebuild(world: &mut World, viewport: Entity) -> Result<()> {
	world
		.run_system_once(
			move |mut commands: Commands,
			      fixture: Res<CharacterMenuState>,
			      focus: Res<InventoryFocusHint>,
			      menus: Query<&HudMenu>| {
				let nodes = fixture.0.menu_nodes();
				if let Some(node) = find_overlay_node(&nodes, "Inventory") {
					let fonts = HudFonts {
						black: default(),
						semibold: default(),
						regular: default(),
						logo: default(),
					};
					fill_inventory_layout(
						&mut commands,
						viewport,
						&fonts,
						node,
						true,
						menus.get(viewport).ok().copied(),
						focus.0,
						&mut NoThumbnails,
					);
				}
			},
		)
		.map_err(|err| anyhow::anyhow!("rebuild: {err}"))?;
	Ok(())
}

fn order(world: &World) -> Result<Vec<usize>> {
	Ok(world
		.resource::<CharacterMenuState>()
		.0
		.inventory
		.as_ref()
		.context("inventory")?
		.clothing
		.clone())
}

fn selected_key(world: &mut World, viewport: Entity) -> Result<usize> {
	let selected = world.get::<HudMenu>(viewport).context("menu")?.selected;
	world
		.query::<(&HudMenuItem, &InventoryItemKey)>()
		.iter(world)
		.find(|(item, _)| item.menu == viewport && item.index == selected)
		.map(|(_, key)| key.index)
		.context("selected item")
}

fn select_item(world: &mut World, viewport: Entity, identity: usize) -> Result<usize> {
	let ordinal = world
		.query::<(&HudMenuItem, &InventoryItemKey)>()
		.iter(world)
		.find(|(item, key)| item.menu == viewport && key.equipped && key.index == identity)
		.map(|(item, _)| item.index)
		.context("item ordinal")?;
	world.get_mut::<HudMenu>(viewport).context("menu")?.selected = ordinal;
	Ok(ordinal)
}

#[test]
fn reorder_from_both_ends_visits_every_slot_and_retains_identity() -> Result<()> {
	for from_end in [false, true] {
		let (mut world, viewport) = fixture();
		rebuild(&mut world, viewport)?;
		let before = order(&world)?;
		let start = if from_end { before.len() - 1 } else { 0 };
		let identity = before[start];
		let original_ordinal = select_item(&mut world, viewport, identity)?;
		world.trigger(MenuNavImpulse::new(viewport, MenuNav::Select));
		world.flush();
		assert_eq!(world.resource::<InventoryInteraction>().reordering(), Some(identity));
		assert!(world.get::<HudMenuNavLocked>(viewport).is_some());
		let nav = if from_end { MenuNav::Left } else { MenuNav::Right };
		for step in 1..before.len() {
			// A stale/hovered ordinal must never select a different bag item's controls.
			world.get_mut::<HudMenu>(viewport).context("menu")?.selected = original_ordinal;
			world.trigger(MenuNavImpulse::new(viewport, nav));
			world.flush();
			rebuild(&mut world, viewport)?;
			let after = order(&world)?;
			let expected = if from_end { start - step } else { step };
			assert_eq!(after.iter().position(|&key| key == identity), Some(expected));
			assert_eq!(selected_key(&mut world, viewport)?, identity);
			assert_eq!(
				after.iter().copied().filter(|&key| key != identity).collect::<Vec<_>>(),
				before.iter().copied().filter(|&key| key != identity).collect::<Vec<_>>()
			);
		}
		let at_end = order(&world)?;
		world.trigger(MenuNavImpulse::new(viewport, nav));
		world.flush();
		world.trigger(MenuNavImpulse::new(viewport, MenuNav::Down));
		world.flush();
		assert_eq!(order(&world)?, at_end);
		world.trigger(MenuNavImpulse::new(viewport, MenuNav::Select));
		world.flush();
		assert_eq!(*world.resource::<InventoryInteraction>(), InventoryInteraction::Browse);
		assert!(world.get::<HudMenuNavLocked>(viewport).is_none());
	}
	Ok(())
}

#[test]
fn standalone_keyboard_moves_the_item_once_without_moving_browse_focus() -> Result<()> {
	let (mut world, viewport) = fixture();
	rebuild(&mut world, viewport)?;
	let before = order(&world)?;
	let original_ordinal = select_item(&mut world, viewport, before[0])?;
	world.trigger(MenuNavImpulse::new(viewport, MenuNav::Select));
	world.flush();
	world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::ArrowRight);
	world
		.run_system_once(navigate_hud_menus)
		.map_err(|err| anyhow::anyhow!("browse: {err}"))?;
	assert_eq!(world.get::<HudMenu>(viewport).context("menu")?.selected, original_ordinal);
	world
		.run_system_once(emit_inventory_reorder_keyboard)
		.map_err(|err| anyhow::anyhow!("keyboard: {err}"))?;
	rebuild(&mut world, viewport)?;
	let mut expected = before.clone();
	expected.swap(0, 1);
	assert_eq!(order(&world)?, expected);
	assert_eq!(selected_key(&mut world, viewport)?, before[0]);
	world.insert_resource(KeyboardMenuNav(false));
	world
		.run_system_once(emit_inventory_reorder_keyboard)
		.map_err(|err| anyhow::anyhow!("routed keyboard: {err}"))?;
	assert_eq!(order(&world)?, expected);
	Ok(())
}

#[test]
fn hover_cannot_steal_reorder_focus_but_click_can_retarget_it() -> Result<()> {
	use bevy::{
		camera::NormalizedRenderTarget,
		picking::{
			backend::HitData,
			pointer::{Location, PointerButton, PointerId},
		},
	};
	use maybraid_character_ui_menu_renderer::input::emit_hud_activate_on_click;
	use menu_components::select_hud_item_on_over;

	let (mut world, viewport) = fixture();
	world.add_observer(select_hud_item_on_over);
	world.add_observer(emit_hud_activate_on_click::<MenuEvent>);
	rebuild(&mut world, viewport)?;
	let before = order(&world)?;
	select_item(&mut world, viewport, before[0])?;
	world.trigger(MenuNavImpulse::new(viewport, MenuNav::Select));
	world.flush();
	let target = world
		.query::<(Entity, &InventoryItemKey)>()
		.iter(&world)
		.find(|(_, key)| key.equipped && key.index == before[1])
		.map(|(entity, _)| entity)
		.context("other tile")?;
	let location = Location {
		target: NormalizedRenderTarget::None { width: 800, height: 600 },
		position: Vec2::ZERO,
	};
	let hit = HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
	world.trigger(PointerOver {
		entity: target,
		pointer: Pointer::new(PointerId::Mouse, location.clone()),
		hit: hit.clone(),
	});
	world.flush();
	assert_eq!(selected_key(&mut world, viewport)?, before[0]);
	world.trigger(PointerClick {
		entity: target,
		pointer: Pointer::new(PointerId::Mouse, location),
		button: PointerButton::Primary,
		hit,
		duration: std::time::Duration::ZERO,
		count: 1,
	});
	world.flush();
	assert_eq!(selected_key(&mut world, viewport)?, before[1]);
	assert_eq!(world.resource::<InventoryInteraction>().reordering(), Some(before[1]));
	world.trigger(MenuNavImpulse::new(viewport, MenuNav::Select));
	world.flush();
	assert_eq!(*world.resource::<InventoryInteraction>(), InventoryInteraction::Browse);
	Ok(())
}

#[test]
fn lubikan_reorder_uses_stable_identity_and_discard_restores_order() -> Result<()> {
	use crate::character::CharacterEditBaseline;
	use lubikans::{LubikanDefinition, LubikanDefinitionId, LubikanLedger, LubikanOrder};
	let (mut world, viewport) = fixture();
	let mut ledger = LubikanLedger::default();
	let first =
		ledger.ensure_owned(LubikanDefinition::new(LubikanDefinitionId::from_raw(9), 20.0, 5.0));
	let last =
		ledger.ensure_owned(LubikanDefinition::new(LubikanDefinitionId::from_raw(2), 20.0, 5.0));
	world.resource_mut::<CharacterMenuState>().0.lubikans = LubikanOrder::from_ledger(&ledger);
	let menu = &world.resource::<CharacterMenuState>().0;
	let baseline = CharacterEditBaseline::capture(menu);
	let identity = character_creation_menus::InventoryLayout::lubikan_item_key(
		menu.inventory.as_ref().context("bag")?,
		&menu.lubikans,
		last,
	)
	.context("account key")?;
	rebuild(&mut world, viewport)?;
	select_item(&mut world, viewport, identity)?;
	world.trigger(MenuNavImpulse::new(viewport, MenuNav::Select));
	world.flush();
	for expected in [
		vec![lubikans::LubikanAccountId::SELF, last, first],
		vec![last, lubikans::LubikanAccountId::SELF, first],
	] {
		world.trigger(MenuNavImpulse::new(viewport, MenuNav::Left));
		world.flush();
		rebuild(&mut world, viewport)?;
		assert_eq!(world.resource::<CharacterMenuState>().0.lubikans.accounts(), expected);
		assert_eq!(selected_key(&mut world, viewport)?, identity);
	}
	assert!(baseline.is_dirty(&world.resource::<CharacterMenuState>().0));
	baseline.restore_into(&mut world.resource_mut::<CharacterMenuState>().0);
	assert_eq!(
		world.resource::<CharacterMenuState>().0.lubikans.accounts(),
		LubikanOrder::from_ledger(&ledger).accounts()
	);
	assert!(!baseline.is_dirty(&world.resource::<CharacterMenuState>().0));
	Ok(())
}
