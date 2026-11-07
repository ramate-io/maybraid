//! Exercise the complete inventory screen tree through the registered renderer input observers.

use anyhow::{Context as ContextExt, Result};
use bevy::{
	asset::{AssetApp, AssetPlugin},
	ecs::system::RunSystemOnce,
	prelude::*,
};
use character_creation_menus::{CharacterMenu, MenuEvent};
use character_items::{ClothingMaterial, ClothingMesh, FirearmMesh, InventoryItem, ItemColor};
use character_ui_menu::MenuComponent;
use maybraid_character_ui_menu_renderer::{
	fill_inventory_screen, InventoryAIOPSlot, InventoryFocusHint, InventoryInteraction,
	InventoryItemKey, InventoryMenuSession, InventoryScreenArea,
	MaybraidCharacterMenuRendererPlugin, NoThumbnails, OverlayClose, OverlaySelectViewport,
};
use menu_components::{
	HudFonts, HudMenu, HudMenuItem, HudOverlayMenu, InventoryNavigation,
	InventoryNavigationImpulse, MenuActivate, MenuComponentsPlugin, MenuNav, MenuNavImpulse,
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
		if menu
			.inventory
			.as_ref()
			.is_some_and(|inventory| inventory.clothing.contains(&index))
		{
			continue;
		}
		if !menu.apply(MenuEvent::EquipInventory(index)) {
			anyhow::bail!("fixture clothing item {index} should equip");
		}
	}

	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_plugins(bevy::input::InputPlugin);
	app.add_plugins(bevy::window::WindowPlugin { primary_window: None, ..default() });
	app.add_plugins(AssetPlugin::default());
	app.init_asset::<Image>();
	app.init_resource::<bevy::picking::hover::HoverMap>();
	app.add_plugins(MenuComponentsPlugin);
	app.add_plugins(MaybraidCharacterMenuRendererPlugin::<MenuEvent>::default());
	app.insert_resource(CharacterMenuState(menu));
	app.init_resource::<CharacterUiSyncState>();
	app.init_resource::<OverlaySelectState>();
	app.init_resource::<OverlayUiSyncState>();
	app.init_resource::<ActiveOverlayKey>();
	app.init_resource::<InventoryInteraction>();
	app.init_resource::<InventoryFocusHint>();
	app.init_resource::<OverlayCloseActivations>();
	app.init_resource::<Messages<CharacterMenuEvent<MenuEvent>>>();
	app.add_observer(on_menu_activate);
	app.add_observer(record_overlay_close);

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

#[derive(Resource, Default)]
struct InventoryEquipActivations(Vec<usize>);

#[derive(Resource, Default)]
struct InventoryMoveActivations(Vec<(usize, i8)>);

#[derive(Resource, Default)]
struct OverlayCloseActivations(usize);

fn record_inventory_equip(
	event: On<MenuActivate<MenuEvent>>,
	mut activations: ResMut<InventoryEquipActivations>,
) {
	if let MenuEvent::EquipInventory(index) = event.choice {
		activations.0.push(index);
	}
}

fn record_inventory_move(
	event: On<MenuActivate<MenuEvent>>,
	mut activations: ResMut<InventoryMoveActivations>,
) {
	if let MenuEvent::MoveEquippedInventory { index, delta } = event.choice {
		activations.0.push((index, delta));
	}
}

fn record_overlay_close(
	_event: On<OverlayClose>,
	mut activations: ResMut<OverlayCloseActivations>,
) {
	activations.0 += 1;
}

#[test]
fn aiop_activation_opens_drawer_back_cancels_and_left_right_still_reorders() -> Result<()> {
	let (mut app, viewport) = fixture()?;
	app.init_resource::<InventoryEquipActivations>();
	app.init_resource::<InventoryMoveActivations>();
	app.add_observer(record_inventory_equip);
	app.add_observer(record_inventory_move);
	let initial_order = clothing_order(app.world())?;
	let source_index = *initial_order.get(1).context("second equipped item")?;
	let source = InventoryItemKey {
		index: source_index,
		equipped: true,
		region: character_ui_menu::InventoryRegion::Equipped,
	};
	app.world_mut()
		.get_mut::<InventoryMenuSession>(viewport)
		.context("session")?
		.region = character_ui_menu::InventoryRegion::Equipped;
	render(&mut app, viewport, true)?;
	let [drawer, equipped] = screen_slots(app.world_mut())?;
	let (source_ordinal, _) = tile(app.world_mut(), equipped, source_index)?;
	app.world_mut()
		.get_mut::<HudMenu>(equipped)
		.context("Equipped HUD menu")?
		.selected = source_ordinal;
	app.world_mut().trigger(MenuNavImpulse::new(equipped, MenuNav::Select));
	app.world_mut().flush();
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Drawer);
	assert_eq!(session.picker_source.map(|source| source.item), Some(Some(source)));
	assert!(app.world().get::<HudOverlayMenu>(drawer).is_some());
	assert!(app.world().get::<HudOverlayMenu>(equipped).is_none());

	app.world_mut()
		.trigger(InventoryNavigationImpulse::new(equipped, InventoryNavigation::ToggleRegion));
	app.world_mut().flush();
	app.world_mut()
		.trigger(InventoryNavigationImpulse::new(equipped, InventoryNavigation::NextSection));
	app.world_mut().flush();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Drawer);
	assert_eq!(session.section, character_ui_menu::InventorySection::Clothing);

	// Back cancels the chooser and leaves the inventory screen open at its source row.
	app.world_mut().trigger(MenuNavImpulse::new(drawer, MenuNav::Back));
	app.world_mut().flush();
	app.update();
	assert!(app.world().get::<OverlaySelectViewport>(viewport).is_some());
	assert_eq!(app.world().resource::<OverlayCloseActivations>().0, 0);
	assert_eq!(
		app.world().get::<InventoryMenuSession>(viewport).context("session")?.region,
		character_ui_menu::InventoryRegion::Equipped
	);
	assert_eq!(focus_identity(app.world(), viewport)?, source);

	// Left/Right still dispatch the existing adjacent-order event from the active AIOP row.
	let stale_focus = InventoryItemKey {
		index: *initial_order.first().context("stale focused item")?,
		equipped: true,
		region: character_ui_menu::InventoryRegion::Equipped,
	};
	app.world_mut()
		.get_mut::<InventoryMenuSession>(viewport)
		.context("session")?
		.active_memory_mut()
		.focused_item = Some(stale_focus);
	let [_, equipped] = screen_slots(app.world_mut())?;
	app.world_mut().trigger(MenuNavImpulse::new(equipped, MenuNav::Left));
	app.world_mut().flush();
	let mut expected = initial_order.clone();
	expected.swap(0, 1);
	assert_eq!(clothing_order(app.world())?, expected);
	assert_eq!(focus_identity(app.world(), viewport)?.index, source_index);
	assert_eq!(app.world().resource::<InventoryMoveActivations>().0, [(source_index, -1)]);
	render(&mut app, viewport, true)?;
	let [_, equipped] = screen_slots(app.world_mut())?;
	let selected = app.world().get::<HudMenu>(equipped).context("Equipped HUD menu")?.selected;
	let mut current_items = app.world_mut().query::<(&HudMenuItem, &InventoryItemKey)>();
	assert!(current_items
		.iter(app.world())
		.any(|(item, key)| { item.menu == equipped && item.index == selected && *key == source }));
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
fn picker_waits_for_rebuilt_drawer_to_acknowledge_one_equip() -> Result<()> {
	let (mut app, viewport) = fixture()?;
	app.init_resource::<InventoryEquipActivations>();
	app.add_observer(record_inventory_equip);
	let source_index = *clothing_order(app.world())?.first().context("equipped source")?;
	app.world_mut()
		.get_mut::<InventoryMenuSession>(viewport)
		.context("session")?
		.region = character_ui_menu::InventoryRegion::Equipped;
	render(&mut app, viewport, true)?;
	let [_, equipped] = screen_slots(app.world_mut())?;
	let (source_ordinal, _) = tile(app.world_mut(), equipped, source_index)?;
	app.world_mut()
		.get_mut::<HudMenu>(equipped)
		.context("Equipped HUD menu")?
		.selected = source_ordinal;
	app.world_mut().trigger(MenuNavImpulse::new(equipped, MenuNav::Select));
	app.world_mut().flush();
	app.update();

	let [drawer, _] = screen_slots(app.world_mut())?;
	let current_order = clothing_order(app.world())?;
	let candidate = (0..5)
		.find(|index| !current_order.contains(index))
		.context("unequipped clothing candidate")?;
	let (candidate_ordinal, _) = tile(app.world_mut(), drawer, candidate)?;
	app.world_mut().get_mut::<HudMenu>(drawer).context("Drawer HUD menu")?.selected =
		candidate_ordinal;
	app.world_mut().trigger(MenuNavImpulse::new(drawer, MenuNav::Select));
	app.world_mut().flush();
	assert_eq!(app.world().resource::<InventoryEquipActivations>().0, [candidate]);
	assert!(clothing_order(app.world())?.contains(&candidate));
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Drawer);
	assert_eq!(session.picker_source.and_then(|source| source.attempted_item), Some(candidate));

	// The host has accepted the action, but the picker waits for the rebuilt tile state.
	render(&mut app, viewport, true)?;
	assert!(app
		.world()
		.get::<InventoryMenuSession>(viewport)
		.context("session")?
		.picker_source
		.is_some());
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert!(session.picker_source.is_none());
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Equipped);
	assert_eq!(focus_identity(app.world(), viewport)?.index, source_index);
	Ok(())
}

#[test]
fn rejected_full_capacity_choice_remains_in_the_drawer() -> Result<()> {
	let (mut app, viewport) = fixture()?;
	let mut full_menu = CharacterMenu::for_create(
		(0..8)
			.map(|_| {
				InventoryItem::clothing(
					ClothingMesh::Pants,
					ClothingMaterial::Cloth,
					ItemColor::Red,
				)
			})
			.collect(),
	);
	for index in 0..6 {
		if full_menu
			.inventory
			.as_ref()
			.is_some_and(|inventory| inventory.clothing.contains(&index))
		{
			continue;
		}
		if !full_menu.apply(MenuEvent::EquipInventory(index)) {
			anyhow::bail!("fixture clothing item {index} should equip");
		}
	}
	app.insert_resource(CharacterMenuState(full_menu));
	app.init_resource::<InventoryEquipActivations>();
	app.add_observer(record_inventory_equip);
	let candidate = 6;
	let source_index = *clothing_order(app.world())?.first().context("equipped source")?;
	app.world_mut()
		.get_mut::<InventoryMenuSession>(viewport)
		.context("session")?
		.region = character_ui_menu::InventoryRegion::Equipped;
	render(&mut app, viewport, true)?;
	let [_, equipped] = screen_slots(app.world_mut())?;
	let (source_ordinal, _) = tile(app.world_mut(), equipped, source_index)?;
	app.world_mut()
		.get_mut::<HudMenu>(equipped)
		.context("Equipped HUD menu")?
		.selected = source_ordinal;
	app.world_mut().trigger(MenuNavImpulse::new(equipped, MenuNav::Select));
	app.world_mut().flush();
	app.update();
	let [drawer, _] = screen_slots(app.world_mut())?;
	let (candidate_ordinal, _) = tile(app.world_mut(), drawer, candidate)?;
	app.world_mut().get_mut::<HudMenu>(drawer).context("Drawer HUD menu")?.selected =
		candidate_ordinal;
	app.world_mut().trigger(MenuNavImpulse::new(drawer, MenuNav::Select));
	app.world_mut().flush();
	assert_eq!(app.world().resource::<InventoryEquipActivations>().0, [candidate]);
	assert_eq!(clothing_order(app.world())?.len(), 6);
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Drawer);
	assert_eq!(session.picker_source.and_then(|source| source.attempted_item), Some(candidate));
	assert!(app.world().get::<OverlaySelectViewport>(viewport).is_some());
	Ok(())
}

#[test]
fn choosing_equipped_duplicate_finishes_picker_without_domain_action() -> Result<()> {
	let (mut app, viewport) = fixture()?;
	app.init_resource::<InventoryEquipActivations>();
	app.add_observer(record_inventory_equip);
	let before = clothing_order(app.world())?;
	let source_index = *before.first().context("equipped source")?;
	app.world_mut()
		.get_mut::<InventoryMenuSession>(viewport)
		.context("session")?
		.region = character_ui_menu::InventoryRegion::Equipped;
	render(&mut app, viewport, true)?;
	let [_, equipped] = screen_slots(app.world_mut())?;
	let (source_ordinal, _) = tile(app.world_mut(), equipped, source_index)?;
	app.world_mut()
		.get_mut::<HudMenu>(equipped)
		.context("Equipped HUD menu")?
		.selected = source_ordinal;
	app.world_mut().trigger(MenuNavImpulse::new(equipped, MenuNav::Select));
	app.world_mut().flush();
	app.update();
	render(&mut app, viewport, true)?;
	let [drawer, _] = screen_slots(app.world_mut())?;
	let (duplicate_ordinal, _) = tile(app.world_mut(), drawer, source_index)?;
	app.world_mut().get_mut::<HudMenu>(drawer).context("Drawer HUD menu")?.selected =
		duplicate_ordinal;
	app.world_mut().trigger(MenuNavImpulse::new(drawer, MenuNav::Select));
	app.world_mut().flush();
	assert_eq!(clothing_order(app.world())?, before);
	assert!(app.world().resource::<InventoryEquipActivations>().0.is_empty());
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert!(session.picker_source.is_none());
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Equipped);
	assert_eq!(focus_identity(app.world(), viewport)?.index, source_index);
	assert!(app.world().get::<OverlaySelectViewport>(viewport).is_some());
	Ok(())
}

#[test]
fn empty_equipped_slot_picker_restores_its_ordinal_after_back() -> Result<()> {
	use bevy::{
		camera::NormalizedRenderTarget,
		picking::{
			backend::HitData,
			pointer::{Location, PointerButton, PointerId},
		},
	};

	let (mut app, viewport) = fixture()?;
	{
		let mut session =
			app.world_mut().get_mut::<InventoryMenuSession>(viewport).context("session")?;
		session.section = character_ui_menu::InventorySection::Tools;
		session.region = character_ui_menu::InventoryRegion::Equipped;
	}
	render(&mut app, viewport, true)?;
	let [_, equipped] = screen_slots(app.world_mut())?;
	let order_len = app
		.world()
		.resource::<CharacterMenuState>()
		.0
		.inventory
		.as_ref()
		.context("inventory")?
		.weapons
		.len();
	let mut slots = app.world_mut().query::<(&HudMenuItem, &InventoryAIOPSlot)>();
	let ordinal = slots
		.iter(app.world())
		.filter(|(item, slot)| item.menu == equipped && slot.ordinal >= order_len)
		.map(|(_, slot)| slot.ordinal)
		.min()
		.context("empty equipped slot after the occupied range")?;
	app.world_mut()
		.get_mut::<HudMenu>(equipped)
		.context("Equipped HUD menu")?
		.selected = ordinal;
	let stale_selection = if ordinal == 0 { 1 } else { 0 };
	app.world_mut().trigger(MenuNavImpulse::new(equipped, MenuNav::Select));
	app.world_mut().flush();
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	let source = session.picker_source.context("empty-slot picker")?;
	assert_eq!(source.ordinal, ordinal);
	assert!(source.item.is_none());
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Drawer);
	let other_empty = {
		let mut slots = app.world_mut().query::<(Entity, &HudMenuItem, &InventoryAIOPSlot)>();
		slots
			.iter(app.world())
			.find(|(_, item, slot)| item.menu == equipped && slot.ordinal != ordinal)
			.map(|(entity, _, _)| entity)
			.context("another empty equipped slot")?
	};
	let location = Location {
		target: NormalizedRenderTarget::None { width: 800, height: 600 },
		position: Vec2::ZERO,
	};
	app.world_mut().trigger(PointerClick {
		entity: other_empty,
		pointer: Pointer::new(PointerId::Mouse, location),
		button: PointerButton::Primary,
		hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
		duration: std::time::Duration::ZERO,
		count: 1,
	});
	app.world_mut().flush();
	assert_eq!(
		app.world()
			.get::<InventoryMenuSession>(viewport)
			.context("session")?
			.picker_source
			.map(|source| source.ordinal),
		Some(ordinal),
	);
	app.world_mut()
		.get_mut::<HudMenu>(equipped)
		.context("inactive Equipped HUD menu")?
		.selected = stale_selection;

	let [drawer, _] = screen_slots(app.world_mut())?;
	app.world_mut().trigger(MenuNavImpulse::new(drawer, MenuNav::Back));
	app.world_mut().flush();
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert!(session.picker_source.is_none());
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Equipped);
	assert_eq!(session.active_memory().focused_ordinal, ordinal);
	assert_eq!(
		app.world().get::<HudMenu>(equipped).context("Equipped HUD menu")?.selected,
		ordinal
	);
	assert_eq!(app.world().resource::<OverlayCloseActivations>().0, 0);
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

	app.world_mut()
		.get_mut::<InventoryMenuSession>(viewport)
		.context("session")?
		.region = character_ui_menu::InventoryRegion::Equipped;
	render(&mut app, viewport, false)?;
	let equipped = screen_slots(app.world_mut())?[1];
	let (ordinal, _) = tile(app.world_mut(), equipped, 1)?;
	app.world_mut()
		.get_mut::<HudMenu>(equipped)
		.context("Equipped HUD menu")?
		.selected = ordinal;
	app.world_mut().trigger(MenuNavImpulse::new(equipped, MenuNav::Left));
	app.world_mut().flush();
	assert_eq!(app.world().resource::<CharacterMenuState>().0, before);
	Ok(())
}
