use crate::MenuController;
use bevy::prelude::*;
use maybraid_controllers_core::{
	CaptureKind, CaptureToken, InputRouter, InputSource, ScopeOwner, UiPadEvent, UiTarget,
};
use maybraid_virtual_pad_controller::{PadButton, VirtualPad, VirtualPadEvent};
use menu_components::{
	HudMenu, MenuAction, MenuActionImpulse, MenuNav, MenuNavImpulse, ShortTextModal,
	TextMenuInputLock,
};

#[derive(Resource, Default)]
struct Hits {
	nav: Vec<(Entity, MenuNav)>,
	actions: Vec<(Entity, MenuAction)>,
	inventory: Vec<(Entity, menu_components::InventoryNavigation)>,
}
fn nav(event: On<MenuNavImpulse>, menus: Query<(), With<HudMenu>>, mut hits: ResMut<Hits>) {
	if menus.contains(event.entity) {
		hits.nav.push((event.entity, event.nav));
	}
}
fn action(event: On<MenuActionImpulse>, mut hits: ResMut<Hits>) {
	hits.actions.push((event.entity, event.action));
}
fn inventory_action(
	event: On<menu_components::InventoryNavigationImpulse>,
	mut hits: ResMut<Hits>,
) {
	hits.inventory.push((event.entity, event.action));
}
fn world() -> World {
	let mut world = World::new();
	world.init_resource::<InputRouter>();
	world.init_resource::<Hits>();
	world.add_observer(crate::dispatch::interpret_menu_pad);
	world.add_observer(nav);
	world.add_observer(action);
	world.add_observer(inventory_action);
	world
}

#[test]
fn inventory_shortcuts_are_scoped_and_back_wins() -> anyhow::Result<()> {
	use menu_components::{InventoryHudMenu, InventoryNavigation, MenuModalLayer, MenuSession};
	let mut world = world();
	let (source, target, _) = menu(&mut world);
	world.entity_mut(target).insert(InventoryHudMenu);
	send(&mut world, source, target, 1, &buttons(&[PadButton::BumperFocus]), 0.0)?;
	assert_eq!(
		world.resource::<Hits>().inventory,
		[(target, InventoryNavigation::PreviousSection)]
	);
	let mut page_down = VirtualPad::default();
	page_down.keys.press(KeyCode::PageDown);
	send(&mut world, source, target, 2, &page_down, 0.0)?;
	assert_eq!(
		world.resource::<Hits>().inventory.last(),
		Some(&(target, InventoryNavigation::NextSection))
	);
	let back_and_next = buttons(&[PadButton::B, PadButton::BumperFire]);
	send(&mut world, source, target, 3, &back_and_next, 0.0)?;
	assert_eq!(world.resource::<Hits>().inventory.len(), 2);
	assert_eq!(world.resource::<Hits>().nav.last(), Some(&(target, MenuNav::Back)));

	// A move button can't switch inventory context while reorder owns the menu.
	world.entity_mut(target).insert(menu_components::HudMenuNavLocked);
	send(&mut world, source, target, 4, &buttons(&[PadButton::BumperFire]), 0.0)?;
	assert_eq!(world.resource::<Hits>().inventory.len(), 2);
	world.entity_mut(target).remove::<menu_components::HudMenuNavLocked>();

	// Confirmation modals suppress shell actions while preserving their routed frame.
	let root = world
		.get::<ChildOf>(target)
		.ok_or_else(|| anyhow::anyhow!("controller root"))?
		.parent();
	world.entity_mut(root).insert(MenuSession::default());
	world
		.get_mut::<MenuSession>(root)
		.ok_or_else(|| anyhow::anyhow!("menu session"))?
		.modals
		.open(MenuModalLayer::ITEM_DROP);
	world
		.get_mut::<MenuController>(root)
		.ok_or_else(|| anyhow::anyhow!("menu controller"))?
		.context_blocked = true;
	send(&mut world, source, target, 5, &buttons(&[PadButton::BumperFire]), 0.0)?;
	assert_eq!(world.resource::<Hits>().inventory.len(), 2);
	// A regular menu receiving the same edge has no inventory side effect.
	let (other_source, other_target, _) = menu(&mut world);
	send(&mut world, other_source, other_target, 1, &buttons(&[PadButton::BumperFocus]), 0.0)?;
	assert_eq!(world.resource::<Hits>().inventory.len(), 2);
	Ok(())
}
fn menu(world: &mut World) -> (InputSource, Entity, CaptureToken) {
	let source = InputSource::virtual_pad(
		world
			.spawn(maybraid_virtual_pad_controller::VirtualPadController::new(Default::default()))
			.id(),
	);
	let target = world.spawn(HudMenu::new(3)).id();
	let mut controller = MenuController::for_source(source);
	controller.set_focus(Some(target));
	let root = world.spawn(controller).id();
	world.entity_mut(root).add_child(target);
	let token = world.resource_mut::<InputRouter>().acquire_capture(
		source,
		ScopeOwner::ui(UiTarget::new(target)),
		CaptureKind::MenuNav,
	);
	(source, target, token)
}
fn send(
	world: &mut World,
	source: InputSource,
	target: Entity,
	sequence: u64,
	pad: &VirtualPad,
	delta: f32,
) -> anyhow::Result<()> {
	let frame = VirtualPadEvent {
		source: source.entity().ok_or_else(|| anyhow::anyhow!("source entity"))?,
		sequence,
		pad: pad.clone(),
		delta: std::time::Duration::from_secs_f32(delta),
	};
	let input = world
		.resource::<InputRouter>()
		.route_ui(source, frame)
		.ok_or_else(|| anyhow::anyhow!("ui route"))?;
	world.trigger(UiPadEvent { entity: target, input });
	world.flush();
	Ok(())
}
fn buttons(buttons: &[PadButton]) -> VirtualPad {
	let mut pad = VirtualPad::default();
	pad.begin_frame();
	for button in buttons {
		pad.hold_digital(*button);
	}
	pad.finish_digital();
	pad
}
#[test]
fn two_sources_repeat_independently_despite_an_unrelated_primary_text_lock() -> anyhow::Result<()> {
	let mut world = world();
	let (first, first_menu, _) = menu(&mut world);
	let (second, second_menu, _) = menu(&mut world);
	world.insert_resource(TextMenuInputLock(true));
	world.insert_resource(ShortTextModal { dismissed: true, ..default() });
	let mut held = VirtualPad::default();
	held.move_stick = Vec2::new(0.0, -1.0);
	send(&mut world, first, first_menu, 1, &held, 0.0)?;
	send(&mut world, first, first_menu, 2, &held, 0.4)?;
	send(&mut world, second, second_menu, 1, &held, 0.0)?;
	assert_eq!(
		world.resource::<Hits>().nav,
		[(first_menu, MenuNav::Down), (first_menu, MenuNav::Down), (second_menu, MenuNav::Down)]
	);
	// Same sequence must not replay an edge or advance repeat.
	send(&mut world, second, second_menu, 1, &held, 0.4)?;
	assert_eq!(world.resource::<Hits>().nav.len(), 3);
	Ok(())
}
#[test]
fn context_edges_are_targeted_one_shot_and_lose_to_navigation() -> anyhow::Result<()> {
	let mut world = world();
	let (first, first_menu, _) = menu(&mut world);
	let (second, second_menu, _) = menu(&mut world);
	let mut pad = buttons(&[PadButton::X]);
	send(&mut world, second, second_menu, 1, &pad, 0.0)?;
	pad.begin_frame();
	pad.hold_digital(PadButton::X);
	pad.finish_digital();
	send(&mut world, second, second_menu, 2, &pad, 0.1)?;
	assert_eq!(world.resource::<Hits>().actions, [(second_menu, MenuAction::Drop)]);
	for (sequence, chord) in [
		(1, vec![PadButton::A, PadButton::Y]),
		(2, vec![PadButton::B, PadButton::X]),
		(3, vec![PadButton::DpadRight, PadButton::X]),
	] {
		send(&mut world, first, first_menu, sequence, &buttons(&chord), 0.0)?;
	}
	assert_eq!(world.resource::<Hits>().actions.len(), 1);
	send(&mut world, first, first_menu, 4, &buttons(&[PadButton::X, PadButton::Y]), 0.0)?;
	assert_eq!(world.resource::<Hits>().actions.last(), Some(&(first_menu, MenuAction::Unequip)));
	Ok(())
}
#[test]
fn replaced_capture_drops_queued_actions_and_old_holds_until_release() -> anyhow::Result<()> {
	let mut world = world();
	let (source, target, token) = menu(&mut world);
	send(&mut world, source, target, 1, &buttons(&[PadButton::Y]), 0.0)?;
	let queued = world
		.resource::<InputRouter>()
		.route_ui(source, MenuAction::Drop)
		.ok_or_else(|| anyhow::anyhow!("route"))?;
	queued.trigger(&mut world.commands(), move |input| MenuActionImpulse::routed(input));
	world.resource_mut::<InputRouter>().release_capture(token);
	world.resource_mut::<InputRouter>().acquire_capture(
		source,
		ScopeOwner::ui(UiTarget::new(target)),
		CaptureKind::MenuNav,
	);
	world.flush();
	assert_eq!(world.resource::<Hits>().actions, [(target, MenuAction::Unequip)]);
	let mut held = buttons(&[PadButton::Y]);
	held.begin_frame();
	held.hold_digital(PadButton::Y);
	held.finish_digital();
	send(&mut world, source, target, 2, &held, 0.0)?;
	assert_eq!(
		world.resource::<Hits>().actions.len(),
		1,
		"old held input cannot cross the capture boundary"
	);
	send(&mut world, source, target, 3, &VirtualPad::default(), 0.0)?;
	send(&mut world, source, target, 4, &buttons(&[PadButton::X]), 0.0)?;
	assert_eq!(world.resource::<Hits>().actions.last(), Some(&(target, MenuAction::Drop)));
	Ok(())
}

#[test]
fn pause_root_start_reaches_shell_while_a_and_submenu_start_select() -> anyhow::Result<()> {
	let mut world = world();
	let (source, target, _) = menu(&mut world);
	let parent = world.get::<ChildOf>(target).ok_or_else(|| anyhow::anyhow!("parent"))?.parent();
	world
		.get_mut::<MenuController>(parent)
		.ok_or_else(|| anyhow::anyhow!("controller"))?
		.start = crate::MenuStartPolicy::Shell;
	send(&mut world, source, target, 1, &buttons(&[PadButton::Start]), 0.0)?;
	assert!(world.resource::<Hits>().nav.is_empty());
	send(&mut world, source, target, 2, &buttons(&[PadButton::A]), 0.0)?;
	assert_eq!(world.resource::<Hits>().nav, [(target, MenuNav::Select)]);
	world
		.get_mut::<MenuController>(parent)
		.ok_or_else(|| anyhow::anyhow!("controller"))?
		.start = crate::MenuStartPolicy::Select;
	send(&mut world, source, target, 3, &buttons(&[PadButton::Start]), 0.0)?;
	assert_eq!(world.resource::<Hits>().nav.len(), 2);
	Ok(())
}

#[test]
fn scoped_text_sessions_with_the_same_field_key_open_and_dismiss_independently(
) -> anyhow::Result<()> {
	let mut world = world();
	let (first, first_menu, _) = menu(&mut world);
	let (second, second_menu, _) = menu(&mut world);
	let first_root = world
		.get::<ChildOf>(first_menu)
		.ok_or_else(|| anyhow::anyhow!("first parent"))?
		.parent();
	let second_root = world
		.get::<ChildOf>(second_menu)
		.ok_or_else(|| anyhow::anyhow!("second parent"))?
		.parent();
	world.entity_mut(first_root).insert(crate::MenuSession::default());
	world.entity_mut(second_root).insert(crate::MenuSession::default());
	let field = |menu, value: &str| {
		(
			menu_components::HudMenuItem { menu, index: 0 },
			menu_components::ShortTextKey("name"),
			menu_components::ShortTextField { value: value.into(), max_len: 16, editing: false },
			ChildOf(menu),
		)
	};
	let first_field = world.spawn(field(first_menu, "Alpha")).id();
	let second_field = world.spawn(field(second_menu, "Beta")).id();
	send(&mut world, first, first_menu, 1, &buttons(&[PadButton::A]), 0.0)?;
	send(&mut world, second, second_menu, 1, &buttons(&[PadButton::A]), 0.0)?;
	assert!(world
		.get::<crate::MenuSession>(first_root)
		.is_some_and(|session| session.text.is_open()));
	assert!(world
		.get::<crate::MenuSession>(second_root)
		.is_some_and(|session| session.text.is_open()));
	send(&mut world, first, first_menu, 2, &buttons(&[PadButton::B]), 0.0)?;
	assert!(world
		.get::<crate::MenuSession>(first_root)
		.is_some_and(|session| !session.text.is_open() && session.modals.consumed()));
	assert!(world
		.get::<crate::MenuSession>(second_root)
		.is_some_and(|session| session.text.is_open()));
	assert!(world
		.get::<menu_components::ShortTextField>(first_field)
		.is_some_and(|field| !field.editing && field.value == "Alpha"));
	send(&mut world, second, second_menu, 2, &buttons(&[PadButton::Start]), 0.0)?;
	assert!(world
		.get::<menu_components::ShortTextField>(second_field)
		.is_some_and(|field| !field.editing && field.value == "Beta"));
	assert!(
		world.resource::<Hits>().nav.is_empty(),
		"opening, cancel, and Start submit must not also activate rows"
	);
	Ok(())
}

#[test]
fn nested_domain_actions_and_back_requests_keep_their_original_authorization() -> anyhow::Result<()>
{
	let mut world = world();
	let (source, target, token) = menu(&mut world);
	let (_, other, _) = menu(&mut world);
	let action = world
		.resource::<InputRouter>()
		.route_ui(source, MenuAction::Drop)
		.ok_or_else(|| anyhow::anyhow!("action route"))?;
	let impulse = MenuActionImpulse::routed(action);
	impulse.deliver(&mut world.commands(), MenuActionImpulse::new(target, MenuAction::Drop));
	let back = world
		.resource::<InputRouter>()
		.route_ui(source, source.entity())
		.ok_or_else(|| anyhow::anyhow!("back route"))?;
	let back = menu_components::MenuBackRequested::routed(back);
	back.queue(&mut world.commands(), move |world| {
		world.resource_mut::<Hits>().nav.push((target, MenuNav::Back))
	});
	world.resource_mut::<InputRouter>().release_capture(token);
	world.resource_mut::<InputRouter>().acquire_capture(
		source,
		ScopeOwner::ui(UiTarget::new(other)),
		CaptureKind::MenuNav,
	);
	world.flush();
	assert!(world.resource::<Hits>().actions.is_empty());
	assert!(world.resource::<Hits>().nav.is_empty());
	Ok(())
}

#[test]
fn held_direction_waits_for_neutral_after_capture_retirement_without_blocking_another_source(
) -> anyhow::Result<()> {
	let mut world = world();
	let (first, first_menu, token) = menu(&mut world);
	let (second, second_menu, _) = menu(&mut world);
	let mut held = VirtualPad::default();
	held.move_stick = Vec2::new(0.0, -1.0);
	send(&mut world, first, first_menu, 1, &held, 0.0)?;
	world.resource_mut::<InputRouter>().release_capture(token);
	world.resource_mut::<InputRouter>().acquire_capture(
		first,
		ScopeOwner::ui(UiTarget::new(first_menu)),
		CaptureKind::MenuNav,
	);
	send(&mut world, first, first_menu, 2, &held, 1.0)?;
	send(&mut world, second, second_menu, 1, &held, 0.0)?;
	assert_eq!(
		world.resource::<Hits>().nav,
		[(first_menu, MenuNav::Down), (second_menu, MenuNav::Down)]
	);
	send(&mut world, first, first_menu, 3, &VirtualPad::default(), 0.0)?;
	send(&mut world, first, first_menu, 4, &held, 0.0)?;
	assert_eq!(world.resource::<Hits>().nav.last(), Some(&(first_menu, MenuNav::Down)));
	assert_eq!(world.resource::<Hits>().nav.len(), 3);
	Ok(())
}

#[test]
fn primary_root_only_mirrors_its_addressed_source_and_clears_each_frame() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;
	use menu_components::{MenuNavPad, PrimaryMenuInput};
	let mut world = world();
	world.init_resource::<MenuNavPad>();
	let (source, target, _) = menu(&mut world);
	let root = world
		.get::<ChildOf>(target)
		.ok_or_else(|| anyhow::anyhow!("primary parent"))?
		.parent();
	world.entity_mut(root).insert((
		MenuController { focus: Some(UiTarget::new(target)), ..default() },
		PrimaryMenuInput::new(source.entity().ok_or_else(|| anyhow::anyhow!("source"))?),
	));
	let (other_source, other_target, _) = menu(&mut world);
	let other_root = world
		.get::<ChildOf>(other_target)
		.ok_or_else(|| anyhow::anyhow!("secondary parent"))?
		.parent();
	send(&mut world, source, target, 1, &buttons(&[PadButton::DpadDown]), 0.0)?;
	assert!(world
		.get::<PrimaryMenuInput>(root)
		.and_then(PrimaryMenuInput::current_frame)
		.is_some_and(|frame| frame.source == source.entity().unwrap_or(Entity::PLACEHOLDER)));
	assert_eq!(world.resource::<MenuNavPad>().events, [MenuNav::Down]);
	world
		.run_system_once(crate::primary::clear_primary_menu_frame)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	assert!(world
		.get::<PrimaryMenuInput>(root)
		.and_then(PrimaryMenuInput::current_frame)
		.is_none());
	assert!(world.resource::<MenuNavPad>().events.is_empty());
	send(&mut world, other_source, other_target, 1, &buttons(&[PadButton::DpadUp]), 0.0)?;
	assert!(world
		.get::<PrimaryMenuInput>(root)
		.and_then(PrimaryMenuInput::current_frame)
		.is_none());
	assert!(world.get::<PrimaryMenuInput>(other_root).is_none());
	assert!(world.resource::<MenuNavPad>().events.is_empty());
	// A valid route to another subtree cannot update the chosen root, even when
	// the event carries the primary source.
	world.resource_mut::<InputRouter>().acquire_capture(
		source,
		ScopeOwner::ui(UiTarget::new(other_target)),
		CaptureKind::MenuNav,
	);
	send(&mut world, source, other_target, 2, &buttons(&[PadButton::A]), 0.0)?;
	assert!(world
		.get::<PrimaryMenuInput>(root)
		.and_then(PrimaryMenuInput::current_frame)
		.is_none());
	assert!(world.resource::<MenuNavPad>().events.is_empty());
	assert_eq!(
		world.resource::<Hits>().nav,
		[(target, MenuNav::Down), (other_target, MenuNav::Up)]
	);
	Ok(())
}

#[test]
fn primary_host_selection_fails_closed_and_keeps_an_explicit_choice() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;
	use menu_components::{MenuNavPad, PrimaryMenuInput};
	let mut world = World::new();
	world.init_resource::<MenuNavPad>();
	let source = world.spawn_empty().id();
	world.insert_resource(maybraid_virtual_pad_controller::PrimaryVirtualPad(source));
	let first = world.spawn(MenuController::default()).id();
	let second = world.spawn(MenuController::default()).id();
	let scoped = world
		.spawn((
			MenuController::for_source(InputSource::virtual_pad(source)),
			PrimaryMenuInput::new(source),
		))
		.id();
	world
		.run_system_once(crate::primary::sync_primary_menu_host)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	assert!(world.get::<PrimaryMenuInput>(first).is_none());
	assert!(world.get::<PrimaryMenuInput>(second).is_none());
	assert!(world.get::<PrimaryMenuInput>(scoped).is_none());
	world.entity_mut(first).insert(PrimaryMenuInput::new(source));
	world
		.run_system_once(crate::primary::sync_primary_menu_host)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	assert!(world.get::<PrimaryMenuInput>(first).is_some());
	assert!(world.get::<PrimaryMenuInput>(second).is_none());
	// A primary-source change retains the chosen root while clearing its frame.
	let replacement = world.spawn_empty().id();
	world.insert_resource(maybraid_virtual_pad_controller::PrimaryVirtualPad(replacement));
	world
		.run_system_once(crate::primary::sync_primary_menu_host)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	assert!(world
		.get::<PrimaryMenuInput>(first)
		.is_some_and(|input| input.source == replacement && input.current_frame().is_none()));
	Ok(())
}
