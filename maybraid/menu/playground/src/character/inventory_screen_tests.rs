//! Exercise the complete inventory screen tree through the registered renderer input observers.

use anyhow::{Context as ContextExt, Result};
use bevy::{
	asset::{AssetApp, AssetPlugin},
	ecs::system::RunSystemOnce,
	prelude::*,
};
use character_creation_menus::{CharacterMenu, InventoryLayout, MenuEvent};
use character_items::{ClothingMaterial, ClothingMesh, FirearmMesh, InventoryItem, ItemColor};
use character_ui_menu::MenuComponent;
use maybraid_character_ui_menu_renderer::{
	fill_inventory_screen, InventoryAIOPSlot, InventoryFocusHint, InventoryFolderHeader,
	InventoryInteraction, InventoryItemKey, InventoryLeftFocus, InventoryMenuSession,
	InventoryScreenArea, InventorySlotEvent, MaybraidCharacterMenuRendererPlugin, NoThumbnails,
	OverlayClose, OverlaySelectViewport,
};
use menu_components::{
	HudFonts, HudMenu, HudMenuItem, HudOverlayMenu, InventoryNavigation,
	InventoryNavigationImpulse, MenuComponentsPlugin, MenuNav, MenuNavImpulse,
};

use crate::character::{
	inventory_slot_event, on_menu_activate, sync_inventory_overlay, CharacterInventorySession,
	CharacterInventoryViewport, CharacterMenuState, CharacterScreen, CharacterUiSyncState,
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
	app.init_asset::<Font>();
	app.init_resource::<bevy::picking::hover::HoverMap>();
	app.add_plugins(MenuComponentsPlugin);
	app.add_plugins(MaybraidCharacterMenuRendererPlugin::<MenuEvent>::default());
	app.insert_resource(CharacterMenuState(menu));
	app.init_resource::<CharacterUiSyncState>();
	app.init_resource::<OverlaySelectState>();
	app.init_resource::<OverlayUiSyncState>();
	app.init_resource::<CharacterInventorySession>();
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
			CharacterInventoryViewport,
			OverlaySelectViewport,
			maybraid_character_ui_menu_renderer::OverlaySelectRoot,
			InventoryMenuSession::default(),
			InventorySlotEvent(inventory_slot_event),
			Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
			ChildOf(screen),
		))
		.id();
	Ok((app, viewport))
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
struct OverlayCloseActivations(usize);

fn record_overlay_close(mut event: On<OverlayClose>, mut hits: ResMut<OverlayCloseActivations>) {
	// This fixture is the close-event host. Count its delivery once, before bubbling.
	event.propagate(false);
	hits.0 += 1;
}

fn nav(app: &mut App, menu: Entity, action: MenuNav) {
	app.world_mut().trigger(MenuNavImpulse::new(menu, action));
	app.world_mut().flush();
}

fn click(app: &mut App, entity: Entity) {
	use bevy::{
		camera::NormalizedRenderTarget,
		picking::{
			backend::HitData,
			pointer::{Location, PointerButton, PointerId},
		},
	};
	app.world_mut().trigger(PointerClick {
		entity,
		pointer: Pointer::new(
			PointerId::Mouse,
			Location {
				target: NormalizedRenderTarget::None { width: 800, height: 600 },
				position: Vec2::ZERO,
			},
		),
		button: PointerButton::Primary,
		hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
		duration: std::time::Duration::ZERO,
		count: 1,
	});
	app.world_mut().flush();
}

fn start_picker(app: &mut App, viewport: Entity, position: usize) -> Result<Entity> {
	let [drawer, equipped] = screen_slots(app.world_mut())?;
	let section = app.world().get::<InventoryMenuSession>(viewport).context("session")?.section;
	let selected = aiop_slot(app.world_mut(), equipped, section, position)?.0;
	app.world_mut().get_mut::<HudMenu>(equipped).context("AIOP menu")?.selected = selected;
	nav(app, equipped, MenuNav::Select);
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(session.picker_source.map(|source| source.ordinal), Some(position));
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Drawer);
	assert!(app.world().get::<HudOverlayMenu>(drawer).is_some());
	assert!(app.world().get::<HudOverlayMenu>(equipped).is_none());
	Ok(drawer)
}

fn aiop_slot(
	world: &mut World,
	menu: Entity,
	section: character_ui_menu::InventorySection,
	ordinal: usize,
) -> Result<(usize, Entity)> {
	world
		.query::<(Entity, &HudMenuItem, &InventoryAIOPSlot)>()
		.iter(world)
		.find(|(_, item, slot)| {
			item.menu == menu && slot.section == section && slot.ordinal == ordinal
		})
		.map(|(entity, item, _)| (item.index, entity))
		.context("AIOP slot")
}

fn folder(world: &mut World, section: character_ui_menu::InventorySection) -> Result<Entity> {
	world
		.query::<(Entity, &InventoryFolderHeader)>()
		.iter(world)
		.find_map(|(entity, header)| (header.0 == section).then_some(entity))
		.context("folder header")
}

fn open_section(
	app: &mut App,
	viewport: Entity,
	section: character_ui_menu::InventorySection,
) -> Result<()> {
	let mut session =
		app.world_mut().get_mut::<InventoryMenuSession>(viewport).context("session")?;
	session.section = section;
	session.expanded_section = Some(section);
	session.left_focus = InventoryLeftFocus::Header(section);
	Ok(())
}

fn choose_item(app: &mut App, drawer: Entity, index: usize) -> Result<()> {
	let (ordinal, _) = tile(app.world_mut(), drawer, index)?;
	app.world_mut().get_mut::<HudMenu>(drawer).context("Drawer menu")?.selected = ordinal;
	nav(app, drawer, MenuNav::Select);
	Ok(())
}

#[test]
fn aiop_owns_input_until_slot_selection_and_drawer_clicks_cannot_bypass_it() -> Result<()> {
	let (mut app, viewport) = fixture()?;
	render(&mut app, viewport, true)?;
	let before = clothing_order(app.world())?;
	let [drawer, equipped] = screen_slots(app.world_mut())?;
	assert!(app.world().get::<HudOverlayMenu>(equipped).is_some());
	assert!(app.world().get::<HudOverlayMenu>(drawer).is_none());
	assert_eq!(
		app.world().get::<InventoryMenuSession>(viewport).context("session")?.region,
		character_ui_menu::InventoryRegion::Equipped
	);
	let initial_drawer_selection =
		app.world().get::<HudMenu>(drawer).context("Drawer menu")?.selected;
	let (_, candidate) = tile(app.world_mut(), drawer, 4)?;
	click(&mut app, candidate);
	nav(&mut app, drawer, MenuNav::Right);
	nav(&mut app, drawer, MenuNav::Select);
	for action in [InventoryNavigation::ToggleRegion, InventoryNavigation::FocusDrawer] {
		app.world_mut().trigger(InventoryNavigationImpulse::new(drawer, action));
		app.world_mut().flush();
	}
	assert_eq!(
		app.world().get::<HudMenu>(drawer).context("Drawer menu")?.selected,
		initial_drawer_selection
	);
	assert_eq!(clothing_order(app.world())?, before);
	assert!(app
		.world()
		.get::<InventoryMenuSession>(viewport)
		.context("session")?
		.picker_source
		.is_none());

	// Up/Down choose slots. Left/Right neither mutate nor move the AIOP selection.
	nav(&mut app, equipped, MenuNav::Down);
	assert_eq!(app.world().get::<HudMenu>(equipped).context("AIOP menu")?.selected, 1);
	nav(&mut app, equipped, MenuNav::Left);
	nav(&mut app, equipped, MenuNav::Right);
	assert_eq!(app.world().get::<HudMenu>(equipped).context("AIOP menu")?.selected, 1);
	assert_eq!(clothing_order(app.world())?, before);
	nav(&mut app, equipped, MenuNav::Up);
	assert_eq!(app.world().get::<HudMenu>(equipped).context("AIOP menu")?.selected, 0);

	let drawer = start_picker(&mut app, viewport, 1)?;
	let old = app.world().get::<HudMenu>(drawer).context("Drawer menu")?.selected;
	nav(&mut app, drawer, MenuNav::Right);
	assert_eq!(app.world().get::<HudMenu>(drawer).context("Drawer menu")?.selected, (old + 1) % 5);
	nav(&mut app, drawer, MenuNav::Left);
	assert_eq!(app.world().get::<HudMenu>(drawer).context("Drawer menu")?.selected, old);
	nav(&mut app, drawer, MenuNav::Down);
	assert_eq!(app.world().get::<HudMenu>(drawer).context("Drawer menu")?.selected, old);
	assert_eq!(clothing_order(app.world())?, before);
	Ok(())
}

#[test]
fn middle_slot_choice_replaces_exact_slot_and_waits_for_that_slot_in_the_host_rebuild() -> Result<()>
{
	let (mut app, viewport) = fixture()?;
	render(&mut app, viewport, true)?;
	let before = clothing_order(app.world())?;
	let candidate = (0..5).find(|index| !before.contains(index)).context("owned candidate")?;
	let drawer = start_picker(&mut app, viewport, 1)?;
	choose_item(&mut app, drawer, candidate)?;
	let mut expected = before.clone();
	expected[1] = candidate;
	assert_eq!(clothing_order(app.world())?, expected);
	app.update();
	assert!(app
		.world()
		.get::<InventoryMenuSession>(viewport)
		.context("session")?
		.picker_source
		.is_some());
	render(&mut app, viewport, true)?;
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert!(session.picker_source.is_none());
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Equipped);
	assert_eq!(session.active_memory().focused_ordinal, 1);
	assert_eq!(focus_identity(app.world(), viewport)?.index, candidate);
	app.update();
	let equipped = screen_slots(app.world_mut())?[1];
	let selected =
		aiop_slot(app.world_mut(), equipped, character_ui_menu::InventorySection::Clothing, 1)?.0;
	assert_eq!(app.world().get::<HudMenu>(equipped).context("AIOP menu")?.selected, selected);
	Ok(())
}

#[test]
fn full_slot_replacement_and_equipped_candidate_swap_preserve_other_slots() -> Result<()> {
	let items = (0..8)
		.map(|_| {
			InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red)
		})
		.collect();
	let (mut app, viewport) = fixture_with_items(items)?;
	for index in 0..6 {
		let menu = &mut app.world_mut().resource_mut::<CharacterMenuState>().0;
		if !menu
			.inventory
			.as_ref()
			.is_some_and(|inventory| inventory.clothing.contains(&index))
		{
			assert!(menu.apply(MenuEvent::EquipInventory(index)));
		}
	}
	render(&mut app, viewport, true)?;
	let mut expected = clothing_order(app.world())?;
	let drawer = start_picker(&mut app, viewport, 3)?;
	choose_item(&mut app, drawer, 6)?;
	expected[3] = 6;
	assert_eq!(clothing_order(app.world())?, expected);
	render(&mut app, viewport, true)?;
	app.update();
	let drawer = start_picker(&mut app, viewport, 2)?;
	let candidate = expected[5];
	choose_item(&mut app, drawer, candidate)?;
	expected.swap(2, 5);
	assert_eq!(clothing_order(app.world())?, expected);
	// A stale Drawer equipped marker cannot acknowledge a relocation to another slot.
	app.update();
	assert!(app
		.world()
		.get::<InventoryMenuSession>(viewport)
		.context("session")?
		.picker_source
		.is_some());
	render(&mut app, viewport, true)?;
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert!(session.picker_source.is_none());
	assert_eq!(session.active_memory().focused_ordinal, 2);
	assert_eq!(focus_identity(app.world(), viewport)?.index, candidate);
	Ok(())
}

#[test]
fn back_cancels_picker_before_second_back_closes_inventory() -> Result<()> {
	use menu_components::{MenuModalLayer, MenuModalStack, MenuNavPad};
	let (mut app, viewport) = fixture()?;
	app.world_mut().resource_mut::<ActiveOverlayKey>().0 = Some("Inventory");
	app.world_mut()
		.resource_mut::<MenuModalStack>()
		.open(MenuModalLayer::OVERLAY_SELECT);
	render(&mut app, viewport, true)?;
	let before = clothing_order(app.world())?;
	let drawer = start_picker(&mut app, viewport, 2)?;
	nav(&mut app, drawer, MenuNav::Right);
	nav(&mut app, drawer, MenuNav::Back);
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert!(session.picker_source.is_none());
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Equipped);
	assert_eq!(session.active_memory().focused_ordinal, 2);
	assert_eq!(clothing_order(app.world())?, before);
	assert_eq!(app.world().resource::<OverlayCloseActivations>().0, 0);
	// The next fresh Back is allowed through to the existing modal dismissal path.
	app.world_mut().resource_mut::<MenuNavPad>().events.push(MenuNav::Back);
	// Preserve the registered modal systems and their message-reader cursors.
	app.world_mut().run_schedule(Update);
	app.world_mut().flush();
	assert_eq!(app.world().resource::<OverlayCloseActivations>().0, 1);
	Ok(())
}

/// Keep production gamepad sampling, routing, capture, and menu interpretation
/// active; only the platform HID backend is omitted in this headless fixture.
fn routed_gamepad_fixture() -> Result<(App, Entity, Entity)> {
	use maybraid_menu_controller::{MenuController, MenuControllerPlugin};
	use maybraid_virtual_pad_controller::PadHidPolicy;

	let (mut app, viewport) = fixture()?;
	app.insert_resource(PadHidPolicy::External);
	app.add_plugins(MenuControllerPlugin);
	let screen = app.world().get::<ChildOf>(viewport).context("inventory screen")?.parent();
	app.world_mut().entity_mut(screen).insert(MenuController::default());
	let gamepad = app.world_mut().spawn(Gamepad::default()).id();
	app.world_mut().resource_mut::<ActiveOverlayKey>().0 = Some("Inventory");
	render(&mut app, viewport, true)?;
	app.update();
	assert!(!app.world().resource::<menu_components::KeyboardMenuNav>().is_enabled());
	Ok((app, viewport, gamepad))
}

fn gamepad_button(app: &mut App, gamepad: Entity, button: GamepadButton, down: bool) -> Result<()> {
	let mut device = app.world_mut().get_mut::<Gamepad>(gamepad).context("gamepad device")?;
	if down {
		device.digital_mut().press(button);
	} else {
		device.digital_mut().release(button);
	}
	app.update();
	Ok(())
}

fn tap_gamepad(app: &mut App, gamepad: Entity, button: GamepadButton) -> Result<()> {
	gamepad_button(app, gamepad, button, true)?;
	gamepad_button(app, gamepad, button, false)
}

fn drawer_visibility(app: &mut App) -> Result<Visibility> {
	let drawer = screen_slots(app.world_mut())?[0];
	let panel = app.world().get::<ChildOf>(drawer).context("drawer panel parent")?.parent();
	app.world().get::<Visibility>(panel).copied().context("drawer panel visibility")
}

#[test]
fn routed_gamepad_b_cancels_picker_and_holds_until_release_before_closing() -> Result<()> {
	use character_ui_menu::InventoryRegion;
	use maybraid_virtual_pad_controller::{PadButton, PrimaryVirtualPad, VirtualPadController};
	use menu_components::{MenuModalId, MenuModalStack, MenuNavPad};

	let (mut app, viewport, gamepad) = routed_gamepad_fixture()?;
	let before = clothing_order(app.world())?;
	assert_eq!(drawer_visibility(&mut app)?, Visibility::Hidden);
	for _ in 0..3 {
		tap_gamepad(&mut app, gamepad, GamepadButton::DPadDown)?;
	}
	tap_gamepad(&mut app, gamepad, GamepadButton::South)?;
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(session.picker_source.map(|source| source.ordinal), Some(2));
	assert_eq!(session.region, InventoryRegion::Drawer);
	assert_eq!(drawer_visibility(&mut app)?, Visibility::Inherited);

	gamepad_button(&mut app, gamepad, GamepadButton::East, true)?;
	let primary = app.world().resource::<PrimaryVirtualPad>().0;
	assert!(app
		.world()
		.get::<VirtualPadController>(primary)
		.context("primary pad")?
		.pad
		.just_pressed(PadButton::B));
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert!(session.picker_source.is_none());
	assert_eq!(session.region, InventoryRegion::Equipped);
	assert_eq!(session.active_memory().focused_ordinal, 2);
	assert_eq!(clothing_order(app.world())?, before);
	assert_eq!(drawer_visibility(&mut app)?, Visibility::Hidden);
	assert!(app.world().resource::<MenuModalStack>().contains(MenuModalId::OVERLAY_SELECT));
	assert!(!app.world().resource::<MenuNavPad>().just_pressed(MenuNav::Back));
	assert_eq!(app.world().resource::<OverlayCloseActivations>().0, 0);
	// A sustained B must never escape to the outer overlay after cancellation.
	for _ in 0..3 {
		app.update();
		assert_eq!(app.world().resource::<OverlayCloseActivations>().0, 0);
	}
	gamepad_button(&mut app, gamepad, GamepadButton::East, false)?;
	assert_eq!(app.world().resource::<OverlayCloseActivations>().0, 0);
	gamepad_button(&mut app, gamepad, GamepadButton::East, true)?;
	assert_eq!(app.world().resource::<OverlayCloseActivations>().0, 1);
	Ok(())
}

#[test]
fn routed_gamepad_traverses_folder_headers_and_opens_only_the_selected_category() -> Result<()> {
	use character_ui_menu::InventorySection;
	let (mut app, viewport, gamepad) = routed_gamepad_fixture()?;
	let before = clothing_order(app.world())?;
	// Confirm on the focused header closes Clothing without opening the drawer.
	tap_gamepad(&mut app, gamepad, GamepadButton::South)?;
	assert_eq!(
		app.world()
			.get::<InventoryMenuSession>(viewport)
			.context("session")?
			.expanded_section,
		None
	);
	render(&mut app, viewport, true)?;
	// The fixture swaps the tree outside Update; refresh controller capture first.
	app.update();
	tap_gamepad(&mut app, gamepad, GamepadButton::DPadDown)?;
	assert_eq!(
		app.world().get::<InventoryMenuSession>(viewport).context("session")?.left_focus,
		InventoryLeftFocus::Header(InventorySection::Tools)
	);
	let section = app.world().get::<InventoryMenuSession>(viewport).context("session")?.section;
	// Bumpers no longer replace editable inventory categories.
	for button in [GamepadButton::LeftTrigger, GamepadButton::RightTrigger] {
		tap_gamepad(&mut app, gamepad, button)?;
	}
	assert_eq!(
		app.world().get::<InventoryMenuSession>(viewport).context("session")?.section,
		section
	);
	tap_gamepad(&mut app, gamepad, GamepadButton::South)?;
	render(&mut app, viewport, true)?;
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(session.expanded_section, Some(InventorySection::Tools));
	assert!(session.picker_source.is_none());
	assert_eq!(drawer_visibility(&mut app)?, Visibility::Hidden);
	let mut slots = app.world_mut().query::<&InventoryAIOPSlot>();
	assert!(slots.iter(app.world()).all(|slot| slot.section == InventorySection::Tools));
	tap_gamepad(&mut app, gamepad, GamepadButton::DPadDown)?;
	tap_gamepad(&mut app, gamepad, GamepadButton::South)?;
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(session.picker_source.map(|source| source.ordinal), Some(0));
	assert_eq!(drawer_visibility(&mut app)?, Visibility::Inherited);
	tap_gamepad(&mut app, gamepad, GamepadButton::East)?;
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(
		session.left_focus,
		InventoryLeftFocus::Slot { section: InventorySection::Tools, ordinal: 0 }
	);
	assert!(session.picker_source.is_none());
	assert_eq!(clothing_order(app.world())?, before);
	Ok(())
}

#[test]
fn host_rebuilds_folder_expansion_without_rebuilding_for_focus_and_keeps_drawer_collapsed(
) -> Result<()> {
	use character_ui_menu::InventorySection;
	let (mut app, viewport) = fixture()?;
	app.world_mut().resource_mut::<OverlaySelectState>().open = Some("Inventory");
	{
		let mut sync = app.world_mut().resource_mut::<OverlayUiSyncState>();
		sync.open = Some("Inventory");
		sync.wait_started = Some(std::time::Duration::ZERO);
	}
	app.world_mut().resource_mut::<Time>().advance_by(
		character_creation_menus::INVENTORY_CAPTURE_TIMEOUT + std::time::Duration::from_secs(1),
	);
	app.world_mut().resource_mut::<Time<Virtual>>().advance_by(
		character_creation_menus::INVENTORY_CAPTURE_TIMEOUT + std::time::Duration::from_secs(1),
	);
	let mut schedule = Schedule::default();
	schedule.add_systems(sync_inventory_overlay);
	schedule.run(app.world_mut());
	app.world_mut().flush();
	app.update();
	let original = folder(app.world_mut(), InventorySection::Clothing)?;
	let equipped = screen_slots(app.world_mut())?[1];
	nav(&mut app, equipped, MenuNav::Down);
	schedule.run(app.world_mut());
	app.world_mut().flush();
	assert_eq!(folder(app.world_mut(), InventorySection::Clothing)?, original);
	// Re-focusing and clicking the same section header changes only expansion.
	click(&mut app, original);
	schedule.run(app.world_mut());
	app.world_mut().flush();
	assert_ne!(folder(app.world_mut(), InventorySection::Clothing)?, original);
	assert_eq!(app.world_mut().query::<&InventoryAIOPSlot>().iter(app.world()).count(), 0);
	assert_eq!(drawer_visibility(&mut app)?, Visibility::Hidden);
	let tools = folder(app.world_mut(), InventorySection::Tools)?;
	click(&mut app, tools);
	schedule.run(app.world_mut());
	app.world_mut().flush();
	assert!(app.world_mut().query::<&InventoryAIOPSlot>().iter(app.world()).count() > 0);
	assert!(app
		.world_mut()
		.query::<&InventoryAIOPSlot>()
		.iter(app.world())
		.all(|slot| slot.section == InventorySection::Tools));
	let clothing = folder(app.world_mut(), InventorySection::Clothing)?;
	click(&mut app, clothing);
	schedule.run(app.world_mut());
	app.world_mut().flush();
	assert!(app
		.world_mut()
		.query::<&InventoryAIOPSlot>()
		.iter(app.world())
		.all(|slot| slot.section == InventorySection::Clothing));
	let drawer = screen_slots(app.world_mut())?[0];
	let panel = app.world().get::<ChildOf>(drawer).context("drawer panel")?.parent();
	assert_eq!(
		app.world().get::<Node>(panel).context("drawer panel layout")?.display,
		Display::None
	);
	assert_eq!(drawer_visibility(&mut app)?, Visibility::Hidden);
	let before = clothing_order(app.world())?;
	let candidate = (0..5).find(|index| !before.contains(index)).context("owned candidate")?;
	let [drawer, equipped] = screen_slots(app.world_mut())?;
	let slot = aiop_slot(app.world_mut(), equipped, InventorySection::Clothing, 1)?.1;
	click(&mut app, slot);
	app.update();
	let candidate_tile = tile(app.world_mut(), drawer, candidate)?.1;
	click(&mut app, candidate_tile);
	schedule.run(app.world_mut());
	app.world_mut().flush();
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert!(session.picker_source.is_none());
	assert_eq!(
		session.left_focus,
		InventoryLeftFocus::Slot { section: InventorySection::Clothing, ordinal: 1 }
	);
	assert_eq!(clothing_order(app.world())?[1], candidate);
	assert_eq!(drawer_visibility(&mut app)?, Visibility::Hidden);
	Ok(())
}

#[test]
fn routed_gamepad_b_dismisses_higher_modal_without_cancelling_picker() -> Result<()> {
	use menu_components::{MenuModal, MenuModalDismiss, MenuModalLayer, MenuModalStack};

	let (mut app, viewport, gamepad) = routed_gamepad_fixture()?;
	tap_gamepad(&mut app, gamepad, GamepadButton::DPadDown)?;
	tap_gamepad(&mut app, gamepad, GamepadButton::South)?;
	assert!(app
		.world()
		.get::<InventoryMenuSession>(viewport)
		.context("session")?
		.picker_source
		.is_some());
	let screen = app.world().get::<ChildOf>(viewport).context("inventory screen")?.parent();
	app.world_mut().spawn((MenuModal::ITEM_DROP, HudMenu::new(1), ChildOf(screen)));
	app.world_mut().resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
	let mut dismissed = app.world().resource::<Messages<MenuModalDismiss>>().get_cursor_current();
	app.update();
	gamepad_button(&mut app, gamepad, GamepadButton::East, true)?;
	assert!(app
		.world()
		.get::<InventoryMenuSession>(viewport)
		.context("session")?
		.picker_source
		.is_some());
	assert_eq!(drawer_visibility(&mut app)?, Visibility::Inherited);
	assert_eq!(app.world().resource::<OverlayCloseActivations>().0, 0);
	let dismissals = dismissed
		.read(app.world().resource::<Messages<MenuModalDismiss>>())
		.map(|dismiss| dismiss.id)
		.collect::<Vec<_>>();
	assert_eq!(dismissals, [menu_components::MenuModalId::ITEM_DROP]);
	Ok(())
}

#[test]
fn only_next_empty_slot_is_actionable_and_fill_returns_to_its_exact_position() -> Result<()> {
	let (mut app, viewport) = fixture()?;
	render(&mut app, viewport, true)?;
	let before = clothing_order(app.world())?;
	let equipped = screen_slots(app.world_mut())?[1];
	assert_eq!(
		app.world().get::<HudMenu>(equipped).context("AIOP menu")?.item_count,
		before.len() + 1 + character_ui_menu::InventorySection::ALL.len()
	);
	let mut slots = app.world_mut().query::<(&HudMenuItem, &InventoryAIOPSlot)>();
	assert!(slots
		.iter(app.world())
		.filter(|(item, _)| item.menu == equipped)
		.all(|(_, slot)| slot.ordinal <= before.len()));
	let drawer = start_picker(&mut app, viewport, before.len())?;
	choose_item(&mut app, drawer, before[0])?;
	assert_eq!(clothing_order(app.world())?, before);
	assert_eq!(
		app.world()
			.get::<InventoryMenuSession>(viewport)
			.context("session")?
			.picker_source
			.and_then(|source| source.attempted_item),
		None
	);
	let candidate = (0..5).find(|index| !before.contains(index)).context("owned candidate")?;
	choose_item(&mut app, drawer, candidate)?;
	let mut expected = before.clone();
	expected.push(candidate);
	assert_eq!(clothing_order(app.world())?, expected);
	render(&mut app, viewport, true)?;
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert!(session.picker_source.is_none());
	assert_eq!(session.active_memory().focused_ordinal, before.len());
	assert_eq!(focus_identity(app.world(), viewport)?.index, candidate);
	Ok(())
}

#[test]
fn gallery_navigation_uses_five_columns_after_aiop_opens_the_drawer() -> Result<()> {
	let items = (0..16)
		.map(|_| {
			InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red)
		})
		.collect();
	let (mut app, viewport) = fixture_with_items(items)?;
	render(&mut app, viewport, true)?;
	let drawer = start_picker(&mut app, viewport, 1)?;
	app.world_mut()
		.trigger(InventoryNavigationImpulse::new(drawer, InventoryNavigation::ToggleDrawer));
	app.world_mut().flush();
	{
		let mut session =
			app.world_mut().get_mut::<InventoryMenuSession>(viewport).context("session")?;
		session.active_memory_mut().focused_item = Some(InventoryItemKey {
			index: 4,
			equipped: false,
			region: character_ui_menu::InventoryRegion::Drawer,
		});
	}
	render(&mut app, viewport, true)?;
	let drawer = screen_slots(app.world_mut())?[0];
	for (expected, scroll) in [(9, 0), (14, 1), (15, 2)] {
		nav(&mut app, drawer, MenuNav::Down);
		assert_eq!(app.world().get::<HudMenu>(drawer).context("Drawer menu")?.selected, expected);
		assert_eq!(
			app.world()
				.get::<InventoryMenuSession>(viewport)
				.context("session")?
				.active_memory()
				.gallery_scroll,
			scroll
		);
	}
	let section = app.world().get::<InventoryMenuSession>(viewport).context("session")?.section;
	app.world_mut()
		.trigger(InventoryNavigationImpulse::new(drawer, InventoryNavigation::NextSection));
	app.world_mut().flush();
	assert_eq!(
		app.world().get::<InventoryMenuSession>(viewport).context("session")?.section,
		section
	);
	nav(&mut app, drawer, MenuNav::Back);
	app.update();
	app.world_mut()
		.trigger(InventoryNavigationImpulse::new(drawer, InventoryNavigation::NextSection));
	app.world_mut().flush();
	app.world_mut()
		.trigger(InventoryNavigationImpulse::new(drawer, InventoryNavigation::PreviousSection));
	app.world_mut().flush();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(session.drawer, character_ui_menu::InventoryDrawerMode::Gallery);
	assert_eq!(
		session.section_memory[section.index()][character_ui_menu::InventoryRegion::Drawer.index()]
			.focused_ordinal,
		15
	);
	Ok(())
}

#[test]
fn read_only_screen_blocks_picker_pointer_and_controller_mutations() -> Result<()> {
	let (mut app, viewport) = fixture()?;
	render(&mut app, viewport, false)?;
	let before = app.world().resource::<CharacterMenuState>().0.clone();
	let [drawer, equipped] = screen_slots(app.world_mut())?;
	let (_, candidate) = tile(app.world_mut(), drawer, 4)?;
	click(&mut app, candidate);
	for action in [MenuNav::Select, MenuNav::Left, MenuNav::Right, MenuNav::Up, MenuNav::Down] {
		nav(&mut app, drawer, action);
		nav(&mut app, equipped, action);
	}
	assert!(app
		.world()
		.get::<InventoryMenuSession>(viewport)
		.context("session")?
		.picker_source
		.is_none());
	app.world_mut().trigger(menu_components::MenuActionImpulse::new(
		equipped,
		menu_components::MenuAction::Unequip,
	));
	app.world_mut().flush();
	assert_eq!(app.world().resource::<CharacterMenuState>().0, before);
	Ok(())
}

#[test]
fn rejected_tanz_candidate_allows_retry_in_the_same_slot_picker() -> Result<()> {
	use character_items::TanzId;
	let (mut app, viewport) = fixture()?;
	let start;
	{
		let mut state = app.world_mut().resource_mut::<CharacterMenuState>();
		let inventory = state.0.inventory.as_mut().context("inventory")?;
		let [first, second, valid, ..] = TanzId::STARTER;
		start = inventory.items.len();
		inventory.items.extend([first, second, first, valid].map(InventoryItem::tanz));
		inventory.tanzes = vec![start, start + 1];
	}
	open_section(&mut app, viewport, character_ui_menu::InventorySection::Tanzes)?;
	render(&mut app, viewport, true)?;
	let drawer = start_picker(&mut app, viewport, 1)?;
	choose_item(&mut app, drawer, start + 2)?;
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert_eq!(session.picker_source.and_then(|source| source.attempted_item), None);
	assert_eq!(session.region, character_ui_menu::InventoryRegion::Drawer);
	assert_eq!(
		app.world()
			.resource::<CharacterMenuState>()
			.0
			.inventory
			.as_ref()
			.context("inventory")?
			.tanzes,
		vec![start, start + 1]
	);
	choose_item(&mut app, drawer, start + 3)?;
	assert_eq!(
		app.world()
			.resource::<CharacterMenuState>()
			.0
			.inventory
			.as_ref()
			.context("inventory")?
			.tanzes,
		vec![start, start + 3]
	);
	render(&mut app, viewport, true)?;
	app.update();
	assert!(app
		.world()
		.get::<InventoryMenuSession>(viewport)
		.context("session")?
		.picker_source
		.is_none());
	assert_eq!(focus_identity(app.world(), viewport)?.index, start + 3);
	Ok(())
}

#[test]
fn pointer_back_and_escape_cancel_the_picker_before_dismissal() -> Result<()> {
	for keyboard_back in [false, true] {
		let (mut app, viewport) = fixture()?;
		render(&mut app, viewport, true)?;
		start_picker(&mut app, viewport, 1)?;
		if keyboard_back {
			app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
			// Inject at the Update input boundary, after InputPlugin's edge reset.
			app.world_mut().run_schedule(Update);
			app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::Escape);
		} else {
			let back = app
				.world_mut()
				.query_filtered::<Entity, With<maybraid_character_ui_menu_renderer::CloseOverlaySelect>>(
				)
				.iter(app.world())
				.next()
				.context("footer Back")?;
			click(&mut app, back);
			app.update();
		}
		let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
		assert!(session.picker_source.is_none());
		assert_eq!(session.region, character_ui_menu::InventoryRegion::Equipped);
		assert_eq!(session.active_memory().focused_ordinal, 1);
		assert_eq!(app.world().resource::<OverlayCloseActivations>().0, 0);
	}
	Ok(())
}

#[test]
fn lubikan_slot_picker_swaps_owned_accounts_and_keeps_self_protected() -> Result<()> {
	use lubikans::{
		LubikanAccountId, LubikanDefinition, LubikanDefinitionId, LubikanLedger, LubikanOrder,
	};
	let (mut app, viewport) = fixture()?;
	let mut ledger = LubikanLedger::default();
	let first =
		ledger.ensure_owned(LubikanDefinition::new(LubikanDefinitionId::from_raw(8), 20.0, 5.0));
	let second =
		ledger.ensure_owned(LubikanDefinition::new(LubikanDefinitionId::from_raw(3), 20.0, 5.0));
	app.world_mut().resource_mut::<CharacterMenuState>().0.lubikans =
		LubikanOrder::from_ledger(&ledger);
	open_section(&mut app, viewport, character_ui_menu::InventorySection::Lubikan)?;
	render(&mut app, viewport, true)?;
	let equipped = screen_slots(app.world_mut())?[1];
	let protected =
		aiop_slot(app.world_mut(), equipped, character_ui_menu::InventorySection::Lubikan, 0)?.0;
	app.world_mut().get_mut::<HudMenu>(equipped).context("AIOP menu")?.selected = protected;
	nav(&mut app, equipped, MenuNav::Select);
	assert!(app
		.world()
		.get::<InventoryMenuSession>(viewport)
		.context("session")?
		.picker_source
		.is_none());
	let drawer = start_picker(&mut app, viewport, 1)?;
	let candidate = {
		let state = &app.world().resource::<CharacterMenuState>().0;
		InventoryLayout::lubikan_item_key(
			state.inventory.as_ref().context("inventory")?,
			&state.lubikans,
			second,
		)
		.context("second Lubikan key")?
	};
	choose_item(&mut app, drawer, candidate)?;
	assert_eq!(
		app.world().resource::<CharacterMenuState>().0.lubikans.accounts(),
		&[LubikanAccountId::SELF, second, first]
	);
	app.update();
	assert!(app
		.world()
		.get::<InventoryMenuSession>(viewport)
		.context("session")?
		.picker_source
		.is_some());
	render(&mut app, viewport, true)?;
	app.update();
	let session = app.world().get::<InventoryMenuSession>(viewport).context("session")?;
	assert!(session.picker_source.is_none());
	assert_eq!(session.active_memory().focused_ordinal, 1);
	assert_eq!(focus_identity(app.world(), viewport)?.index, candidate);
	Ok(())
}
