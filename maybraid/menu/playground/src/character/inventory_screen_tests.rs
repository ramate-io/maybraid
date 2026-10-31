//! Exercise the complete inventory screen tree through the registered renderer input observers.

use anyhow::{Context, Result};
use bevy::{
	asset::{AssetApp, AssetPlugin},
	ecs::system::RunSystemOnce,
	prelude::*,
};
use character_creation_menus::{CharacterMenu, MenuEvent};
use character_items::{ClothingMaterial, ClothingMesh, FirearmMesh, InventoryItem, ItemColor};
use character_ui_menu::MenuComponent;
use maybraid_character_ui_menu_renderer::{
	fill_inventory_screen, InventoryFocusHint, InventoryInteraction, InventoryItemKey,
	InventoryMenuSession, InventoryScreenArea, MaybraidCharacterMenuRendererPlugin, NoThumbnails,
	OverlaySelectViewport,
};
use menu_components::{
	HudFonts, HudMenu, HudMenuItem, HudMenuNavLocked, HudOverlayMenu, InventoryHudMenu,
	InventoryNavigation, InventoryNavigationImpulse, MenuComponentsPlugin, MenuNav, MenuNavImpulse,
};

use crate::character::{
	on_menu_activate, CharacterMenuState, CharacterScreen, CharacterUiSyncState,
	OverlaySelectState, OverlayUiSyncState,
};

use maybraid_character_ui_menu_renderer::CharacterMenuEvent;
use menu_components::ActiveOverlayKey;

fn granted_items() -> Vec<InventoryItem> {
	let mut items = (0..5)
		.map(|_| {
			InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red)
		})
		.collect::<Vec<_>>();
	items.push(InventoryItem::firearm(FirearmMesh::Bullpup));
	items.push(InventoryItem::firearm(FirearmMesh::Silopup));
	items
}

fn fixture() -> Result<(App, Entity)> {
	fixture_with_items(granted_items())
}

fn fixture_with_items(items: Vec<InventoryItem>) -> Result<(App, Entity)> {
	let mut menu = CharacterMenu::for_create(items);
	for index in 1..4 {
		if !menu.apply(MenuEvent::EquipInventory(index)) {
			anyhow::bail!("fixture clothing item {index} should equip");
		}
	}

	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_plugins(AssetPlugin::default());
	app.init_asset::<Image>();
	app.add_plugins(MenuComponentsPlugin);
	app.add_plugins(MaybraidCharacterMenuRendererPlugin::<MenuEvent>::default());
	app.insert_resource(CharacterMenuState(menu));
	app.init_resource::<CharacterUiSyncState>();
	app.init_resource::<OverlaySelectState>();
	app.init_resource::<OverlayUiSyncState>();
	app.init_resource::<ActiveOverlayKey>();
	app.init_resource::<InventoryInteraction>();
	app.init_resource::<InventoryFocusHint>();
	app.init_resource::<Messages<CharacterMenuEvent<MenuEvent>>>();
	app.add_observer(on_menu_activate);

	let screen = app.world_mut().spawn(CharacterScreen).id();
	let viewport = app
		.world_mut()
		.spawn((
			OverlaySelectViewport,
			InventoryMenuSession::default(),
			Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
			ChildOf(screen),
		))
		.id();
	Ok((app, viewport))
}

#[test]
fn gallery_navigation_uses_five_columns_and_clamps_the_incomplete_last_row() -> Result<()> {
	let items = (0..16)
		.map(|_| {
			InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red)
		})
		.collect();
	let (mut app, viewport) = fixture_with_items(items)?;
	{
		let mut session = app
			.world_mut()
			.get_mut::<InventoryMenuSession>(viewport)
			.context("inventory session")?;
		session.drawer = character_ui_menu::InventoryDrawerMode::Gallery;
		session.active_memory_mut().focused_item = Some(InventoryItemKey {
			index: 4,
			equipped: false,
			region: character_ui_menu::InventoryRegion::Drawer,
		});
	}
	render(&mut app, viewport, true)?;
	let drawer = screen_slots(app.world_mut())?[0];
	assert_eq!(app.world().get::<HudMenu>(drawer).context("Drawer menu")?.selected, 4);

	for (expected, expected_row) in [(9, 0), (14, 1), (15, 2)] {
		app.world_mut().trigger(MenuNavImpulse::new(drawer, MenuNav::Down));
		app.world_mut().flush();
		assert_eq!(app.world().get::<HudMenu>(drawer).context("Drawer menu")?.selected, expected);
		let session =
			app.world().get::<InventoryMenuSession>(viewport).context("inventory session")?;
		assert_eq!(session.active_memory().gallery_scroll, expected_row);
		let focused = session.active_memory().focused_item;
		let mut items = app.world_mut().query::<(&HudMenuItem, &InventoryItemKey)>();
		let selected_key = items
			.iter(app.world())
			.find(|(item, _)| item.menu == drawer && item.index == expected)
			.map(|(_, key)| *key)
			.context("selected gallery item")?;
		assert_eq!(focused, Some(selected_key));
	}
	Ok(())
}

fn render(app: &mut App, viewport: Entity, interactive: bool) -> Result<()> {
	let result = app
		.world_mut()
		.run_system_once(
			move |mut commands: Commands,
			      assets: Res<AssetServer>,
			      menu: Res<CharacterMenuState>,
			      session: Query<&InventoryMenuSession>| {
				let nodes = menu.0.menu_nodes();
				let node =
					maybraid_character_ui_menu_renderer::find_overlay_node(&nodes, "Inventory")
						.context("inventory node")?;
				let session = session.get(viewport).copied().context("viewport session")?;
				let fonts = HudFonts {
					black: Handle::default(),
					semibold: Handle::default(),
					regular: Handle::default(),
					logo: Handle::default(),
				};
				let mut thumbnails = NoThumbnails;
				fill_inventory_screen(
					&mut commands,
					viewport,
					&fonts,
					&assets,
					menu_components::InputPromptDevice::Keyboard,
					node,
					interactive,
					&session,
					&mut thumbnails,
				);
				Ok::<(), anyhow::Error>(())
			},
		)
		.map_err(|error| anyhow::anyhow!("inventory renderer system: {error}"))?;
	result?;
	app.world_mut().flush();
	Ok(())
}

fn screen_slots(world: &mut World) -> Result<[Entity; 2]> {
	let mut areas = world.query::<(Entity, &InventoryScreenArea, &HudMenu)>();
	let mut drawer = None;
	let mut equipped = None;
	for (entity, area, _) in areas.iter(world) {
		match area.0 {
			character_ui_menu::InventoryRegion::Drawer => drawer = Some(entity),
			character_ui_menu::InventoryRegion::Equipped => equipped = Some(entity),
		}
	}
	Ok([drawer.context("Drawer menu")?, equipped.context("Equipped menu")?])
}

fn tile(world: &mut World, menu: Entity, index: usize) -> Result<(usize, Entity)> {
	let mut items = world.query::<(Entity, &HudMenuItem, &InventoryItemKey)>();
	items
		.iter(world)
		.find(|(_, item, key)| item.menu == menu && key.index == index)
		.map(|(entity, item, _)| (item.index, entity))
		.context("inventory tile")
}

fn clothing_order(world: &World) -> Result<Vec<usize>> {
	Ok(world
		.resource::<CharacterMenuState>()
		.0
		.inventory
		.as_ref()
		.context("character inventory")?
		.clothing
		.clone())
}

fn focus_identity(world: &World, viewport: Entity) -> Result<InventoryItemKey> {
	world
		.get::<InventoryMenuSession>(viewport)
		.context("inventory session")?
		.active_memory()
		.focused_item
		.context("focused inventory item")
}

#[test]
fn checked_drawer_item_enters_aiop_reorder_moves_and_back_cancels_without_closing() -> Result<()> {
	let (mut app, viewport) = fixture()?;
	render(&mut app, viewport, true)?;
	let menus = screen_slots(app.world_mut())?;
	let drawer = menus[0];
	let equipped = menus[1];
	let initial_order = clothing_order(app.world())?;
	let identity = *initial_order.first().context("first equipped item")?;
	let (ordinal, _) = tile(app.world_mut(), drawer, identity)?;
	app.world_mut().get_mut::<HudMenu>(drawer).context("Drawer HUD menu")?.selected = ordinal;
	assert!(app.world().get::<InventoryHudMenu>(drawer).is_some());
	assert!(app.world().get::<InventoryHudMenu>(equipped).is_some());
	assert!(app.world().get::<HudOverlayMenu>(drawer).is_some());
	assert!(app.world().get::<HudOverlayMenu>(equipped).is_none());
	assert_eq!(
		app.world().get::<InventoryMenuSession>(viewport).context("session")?.drawer,
		character_ui_menu::InventoryDrawerMode::Rotary,
	);

	app.world_mut().trigger(MenuNavImpulse::new(drawer, MenuNav::Select));
	app.world_mut().flush();
	assert_eq!(
		app.world().get::<InventoryMenuSession>(viewport).context("session")?.region,
		character_ui_menu::InventoryRegion::Equipped,
	);
	render(&mut app, viewport, true)?;
	let menus = screen_slots(app.world_mut())?;
	let equipped = menus[1];
	assert!(app.world().get::<HudOverlayMenu>(equipped).is_some());
	let (equipped_ordinal, _) = tile(app.world_mut(), equipped, identity)?;
	app.world_mut()
		.get_mut::<HudMenu>(equipped)
		.context("Equipped HUD menu")?
		.selected = equipped_ordinal;
	let focused = focus_identity(app.world(), viewport)?;
	assert_eq!(focused.index, identity);
	assert_eq!(focused.region, character_ui_menu::InventoryRegion::Equipped);
	assert_eq!(clothing_order(app.world())?, initial_order);

	app.world_mut().trigger(MenuNavImpulse::new(equipped, MenuNav::Select));
	app.world_mut().flush();
	assert_eq!(
		app.world()
			.get::<InventoryMenuSession>(viewport)
			.context("session")?
			.interaction,
		InventoryInteraction::Reorder { item: focused },
	);
	assert!(app.world().get::<HudMenuNavLocked>(equipped).is_some());

	app.world_mut().trigger(MenuNavImpulse::new(equipped, MenuNav::Down));
	app.world_mut().flush();
	let moved_order = clothing_order(app.world())?;
	let mut expected = initial_order.clone();
	expected.swap(0, 1);
	assert_eq!(moved_order, expected);
	assert_eq!(focus_identity(app.world(), viewport)?, focused);

	// Rebuild the real screen tree from the updated domain inventory.
	render(&mut app, viewport, true)?;
	let menus = screen_slots(app.world_mut())?;
	let equipped = menus[1];
	let selected = app.world().get::<HudMenu>(equipped).context("rebuilt Equipped menu")?.selected;
	let mut current_items = app.world_mut().query::<(&HudMenuItem, &InventoryItemKey)>();
	let current_focus = current_items
		.iter(app.world())
		.find(|(item, key)| item.menu == equipped && item.index == selected && **key == focused)
		.context("rebuilt focus should keep the moved item")?;
	assert_eq!(current_focus.1.index, identity);
	assert_eq!(
		app.world()
			.get::<InventoryMenuSession>(viewport)
			.context("session")?
			.interaction,
		InventoryInteraction::Reorder { item: focused },
	);

	app.world_mut().trigger(MenuNavImpulse::new(equipped, MenuNav::Back));
	app.world_mut().flush();
	assert_eq!(
		app.world()
			.get::<InventoryMenuSession>(viewport)
			.context("session")?
			.interaction,
		InventoryInteraction::Browse,
	);
	assert!(app.world().get::<OverlaySelectViewport>(viewport).is_some());
	Ok(())
}

#[test]
fn gallery_page_switch_restores_drawer_item_focus() -> Result<()> {
	let (mut app, viewport) = fixture()?;
	render(&mut app, viewport, true)?;
	let drawer = screen_slots(app.world_mut())?[0];
	app.world_mut().trigger(MenuNavImpulse::new(drawer, MenuNav::Right));
	app.world_mut().flush();
	let original_focus = focus_identity(app.world(), viewport)?;
	assert_eq!(original_focus.region, character_ui_menu::InventoryRegion::Drawer);

	app.world_mut()
		.trigger(InventoryNavigationImpulse::new(drawer, InventoryNavigation::ToggleDrawer));
	app.world_mut().flush();
	assert_eq!(
		app.world().get::<InventoryMenuSession>(viewport).context("session")?.drawer,
		character_ui_menu::InventoryDrawerMode::Gallery,
	);
	render(&mut app, viewport, true)?;

	let drawer = screen_slots(app.world_mut())?[0];
	app.world_mut()
		.trigger(InventoryNavigationImpulse::new(drawer, InventoryNavigation::NextSection));
	app.world_mut().flush();
	assert_eq!(
		app.world().get::<InventoryMenuSession>(viewport).context("session")?.section,
		character_ui_menu::InventorySection::Tools,
	);
	render(&mut app, viewport, true)?;

	let drawer = screen_slots(app.world_mut())?[0];
	app.world_mut()
		.trigger(InventoryNavigationImpulse::new(drawer, InventoryNavigation::PreviousSection));
	app.world_mut().flush();
	render(&mut app, viewport, true)?;
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(session.section, character_ui_menu::InventorySection::Clothing);
	assert_eq!(session.drawer, character_ui_menu::InventoryDrawerMode::Gallery);
	assert_eq!(session.active_memory().focused_item, Some(original_focus));
	let drawer = screen_slots(app.world_mut())?[0];
	let selected = app.world().get::<HudMenu>(drawer).context("Drawer HUD menu")?.selected;
	let mut items = app.world_mut().query::<(&HudMenuItem, &InventoryItemKey)>();
	assert!(items.iter(app.world()).any(|(item, key)| {
		item.menu == drawer && item.index == selected && *key == original_focus
	}));
	Ok(())
}

#[test]
fn read_only_screen_cannot_mutate_character_inventory() -> Result<()> {
	let (mut app, viewport) = fixture()?;
	render(&mut app, viewport, false)?;
	let before = app.world().resource::<CharacterMenuState>().0.clone();
	let drawer = screen_slots(app.world_mut())?[0];
	let (ordinal, _) = tile(app.world_mut(), drawer, 4)?;
	app.world_mut().get_mut::<HudMenu>(drawer).context("Drawer HUD menu")?.selected = ordinal;
	app.world_mut().trigger(MenuNavImpulse::new(drawer, MenuNav::Select));
	app.world_mut().flush();
	assert_eq!(app.world().resource::<CharacterMenuState>().0, before);
	Ok(())
}
