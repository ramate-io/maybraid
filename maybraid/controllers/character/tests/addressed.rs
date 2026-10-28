use std::time::Duration;

use anyhow::{anyhow, Result};
use bevy::prelude::*;
use maybraid_character_controller::{
	register_character_controller, ActorControlEvent, ActorControlSample, ActorControlStatus,
	ActorIntentEvent, CharacterIntent, RoutedActorControl,
};
use maybraid_controllers_core::{
	ActorTarget, CaptureKind, InputRouter, InputSource, ScopeOwner, UiTarget,
};
use maybraid_virtual_pad_controller::{
	PadButton, PadDevices, PrimaryVirtualPad, VirtualPad, VirtualPadController, VirtualPadEvent,
};

#[derive(Resource, Default)]
struct Delivered {
	intents: Vec<(Entity, InputSource, CharacterIntent)>,
	controls: Vec<(Entity, InputSource, ActorControlSample)>,
}

fn app() -> App {
	let mut app = App::new();
	register_character_controller(&mut app);
	app.init_resource::<Delivered>()
		.add_observer(|event: On<ActorIntentEvent>, mut delivered: ResMut<Delivered>| {
			delivered
				.intents
				.push((event.entity, event.input.source(), *event.input.payload()));
		})
		.add_observer(|event: On<ActorControlEvent>, mut delivered: ResMut<Delivered>| {
			delivered
				.controls
				.push((event.entity, event.input.source(), *event.input.payload()));
		});
	app
}

fn source(app: &mut App, actor: Entity) -> Entity {
	let source = app.world_mut().spawn(VirtualPadController::new(PadDevices::default())).id();
	app.world_mut()
		.resource_mut::<InputRouter>()
		.assign_actor(InputSource::virtual_pad(source), ActorTarget::new(actor));
	source
}

fn frame(app: &mut App, source: Entity, sequence: u64, pad: VirtualPad) {
	app.world_mut().write_message(VirtualPadEvent {
		source,
		sequence,
		delta: Duration::from_millis(20),
		pad,
	});
}

fn pad(stick: Vec2, buttons: &[PadButton]) -> VirtualPad {
	let mut pad = VirtualPad::default();
	pad.begin_frame();
	pad.add_move(stick);
	for button in buttons {
		pad.hold_digital(*button);
	}
	pad.finish_digital();
	pad
}

#[test]
fn two_sources_keep_actor_state_and_delivery_separate() -> Result<()> {
	let mut app = app();
	let first = app.world_mut().spawn_empty().id();
	let second = app.world_mut().spawn_empty().id();
	let first_source = source(&mut app, first);
	let second_source = source(&mut app, second);
	app.world_mut().insert_resource(PrimaryVirtualPad(first_source));
	app.world_mut().resource_mut::<InputRouter>().set_local_source(first_source);
	frame(&mut app, first_source, 7, pad(Vec2::X, &[PadButton::A]));
	frame(&mut app, second_source, 9, pad(Vec2::NEG_Y, &[PadButton::TriggerFire]));
	app.update();

	let first_control = app
		.world()
		.get::<RoutedActorControl>(first)
		.and_then(RoutedActorControl::payload)
		.ok_or_else(|| anyhow!("first control"))?;
	let second_control = app
		.world()
		.get::<RoutedActorControl>(second)
		.and_then(RoutedActorControl::payload)
		.ok_or_else(|| anyhow!("second control"))?;
	assert_eq!(first_control.move_stick(), Vec2::X);
	assert_eq!(second_control.move_stick(), Vec2::NEG_Y);
	assert_eq!(first_control.delta(), Duration::from_millis(20));
	assert_eq!(first_control.sequence(), 7);
	assert!(first_control.a().just_pressed());
	assert!(!first_control.fire_held());
	assert!(second_control.fire_held());
	assert_eq!(second_control.fire_squeeze(), 0.5);
	let delivered = app.world().resource::<Delivered>();
	assert!(delivered.intents.contains(&(
		first,
		InputSource::virtual_pad(first_source),
		CharacterIntent::Jump
	)));
	assert!(delivered.intents.contains(&(
		second,
		InputSource::virtual_pad(second_source),
		CharacterIntent::UseItem(0.5)
	)));
	let count = delivered.intents.len();
	app.update();
	assert_eq!(app.world().resource::<Delivered>().intents.len(), count);
	assert!(app
		.world()
		.get::<RoutedActorControl>(first)
		.and_then(RoutedActorControl::payload)
		.is_none());
	Ok(())
}

#[test]
fn capture_and_suspension_change_only_the_owned_actor() -> Result<()> {
	let mut app = app();
	let first = app.world_mut().spawn_empty().id();
	let second = app.world_mut().spawn_empty().id();
	let npc = app.world_mut().spawn_empty().id();
	let first_source = source(&mut app, first);
	let second_source = source(&mut app, second);
	let menu = app.world_mut().spawn_empty().id();
	let capture = app.world_mut().resource_mut::<InputRouter>().acquire_capture(
		InputSource::virtual_pad(first_source),
		ScopeOwner::ui(UiTarget::new(menu)),
		CaptureKind::MenuNav,
	);
	frame(&mut app, first_source, 1, pad(Vec2::X, &[PadButton::A, PadButton::Start]));
	frame(&mut app, second_source, 1, pad(Vec2::Y, &[PadButton::A]));
	app.update();
	let first_status = *app
		.world()
		.get::<ActorControlStatus>(first)
		.ok_or_else(|| anyhow!("first status"))?;
	assert!(first_status.is_controlled());
	assert!(first_status.is_captured());
	assert!(!first_status.is_delivered());
	assert!(!first_status.is_frozen());
	assert!(app
		.world()
		.get::<ActorControlStatus>(second)
		.is_some_and(|status| status.is_delivered()));
	assert!(app.world().get::<ActorControlStatus>(npc).is_none());
	let delivered = app.world().resource::<Delivered>();
	assert!(!delivered.controls.iter().any(|(actor, _, _)| *actor == first));
	assert!(delivered.intents.contains(&(
		first,
		InputSource::virtual_pad(first_source),
		CharacterIntent::InGameMenu
	)));
	assert!(!delivered.intents.iter().any(|(actor, _, intent)| *actor == first
		&& matches!(intent, CharacterIntent::Jump | CharacterIntent::Move(_))));
	assert!(delivered.intents.contains(&(
		second,
		InputSource::virtual_pad(second_source),
		CharacterIntent::Jump
	)));
	let suspension = app
		.world_mut()
		.resource_mut::<InputRouter>()
		.suspend(InputSource::virtual_pad(first_source), ScopeOwner::pause());
	app.update();
	assert!(app
		.world()
		.get::<ActorControlStatus>(first)
		.is_some_and(|status| status.is_frozen()));
	assert!(app
		.world()
		.get::<ActorControlStatus>(second)
		.is_some_and(|status| status.is_delivered()));
	app.world_mut().resource_mut::<InputRouter>().resume(suspension);
	app.update();
	assert!(app
		.world()
		.get::<ActorControlStatus>(first)
		.is_some_and(|status| !status.is_frozen() && !status.is_delivered()));
	app.world_mut().resource_mut::<InputRouter>().release_capture(capture);
	app.update();
	assert!(app
		.world()
		.get::<ActorControlStatus>(first)
		.is_some_and(|status| status.is_delivered()));
	app.world_mut()
		.resource_mut::<InputRouter>()
		.disconnect(InputSource::virtual_pad(first_source));
	app.update();
	assert!(app
		.world()
		.get::<ActorControlStatus>(first)
		.is_some_and(|status| !status.is_controlled()
			&& !status.is_connected()
			&& !status.is_frozen()));
	Ok(())
}

#[test]
fn neutral_frames_deliver_releases_and_their_original_clock() -> Result<()> {
	let mut app = app();
	let actor = app.world_mut().spawn_empty().id();
	let source = source(&mut app, actor);
	let mut held = pad(Vec2::X, &[PadButton::A, PadButton::TriggerFire]);
	frame(&mut app, source, 1, held.clone());
	app.update();
	held.begin_frame();
	held.finish_digital();
	frame(&mut app, source, 2, held);
	app.update();
	let sample = app
		.world()
		.resource::<Delivered>()
		.controls
		.last()
		.map(|(_, _, sample)| *sample)
		.ok_or_else(|| anyhow!("neutral sample"))?;
	assert_eq!(sample.move_stick(), Vec2::ZERO);
	assert!(sample.a().just_released());
	assert!(!sample.fire_held());
	assert_eq!(sample.fire_squeeze(), 0.0);
	assert_eq!(sample.sequence(), 2);
	assert_eq!(sample.delta(), Duration::from_millis(20));
	Ok(())
}

#[test]
fn trigger_threshold_belongs_to_the_source_snapshot() -> Result<()> {
	let mut app = app();
	let first = app.world_mut().spawn_empty().id();
	let second = app.world_mut().spawn_empty().id();
	let first_source = source(&mut app, first);
	let second_source = source(&mut app, second);
	app.world_mut()
		.get_mut::<VirtualPadController>(second_source)
		.ok_or_else(|| anyhow!("second controller"))?
		.config
		.trigger_press_threshold = 0.8;
	for source in [first_source, second_source] {
		let threshold = app
			.world()
			.get::<VirtualPadController>(source)
			.ok_or_else(|| anyhow!("source controller"))?
			.config
			.trigger_press_threshold;
		let mut analog = VirtualPad::default();
		analog.begin_frame();
		analog.max_triggers(0.0, 0.6);
		// Inject the completed source snapshot, including its trigger conversion.
		analog.apply_trigger_digital(threshold);
		analog.finish_digital();
		frame(&mut app, source, 1, analog);
	}
	app.update();
	let delivered = app.world().resource::<Delivered>();
	assert!(delivered
		.controls
		.iter()
		.any(|(actor, _, sample)| *actor == first && sample.fire_held()));
	assert!(delivered
		.controls
		.iter()
		.any(|(actor, _, sample)| *actor == second && !sample.fire_held()));
	assert!(delivered
		.intents
		.iter()
		.any(|(actor, _, intent)| *actor == first && *intent == CharacterIntent::UseItem(0.6)));
	assert!(delivered
		.intents
		.iter()
		.any(|(actor, _, intent)| *actor == first && *intent == CharacterIntent::AttackRight));
	assert!(!delivered.intents.iter().any(|(actor, _, intent)| *actor == second
		&& matches!(intent, CharacterIntent::UseItem(_) | CharacterIntent::AttackRight)));
	Ok(())
}

#[derive(Resource)]
struct CaptureOnControl {
	source: InputSource,
	owner: UiTarget,
}

fn capture_on_control(
	event: On<ActorControlEvent>,
	capture: Res<CaptureOnControl>,
	mut routing: maybraid_controllers_core::InputRouting,
) {
	if event.input.source() == capture.source {
		routing.acquire_capture(
			capture.source,
			ScopeOwner::ui(capture.owner),
			CaptureKind::MenuNav,
		);
	}
}

#[test]
fn capture_during_control_delivery_retires_queued_semantics_and_cached_state() -> Result<()> {
	let mut app = app();
	let first = app.world_mut().spawn_empty().id();
	let second = app.world_mut().spawn_empty().id();
	let first_source = source(&mut app, first);
	let second_source = source(&mut app, second);
	let menu = app.world_mut().spawn_empty().id();
	app.insert_resource(CaptureOnControl {
		source: InputSource::virtual_pad(first_source),
		owner: UiTarget::new(menu),
	})
	.add_observer(capture_on_control);
	frame(&mut app, first_source, 1, pad(Vec2::X, &[PadButton::A, PadButton::TriggerFire]));
	frame(&mut app, second_source, 1, pad(Vec2::Y, &[PadButton::A]));
	app.update();
	let delivered = app.world().resource::<Delivered>();
	assert!(!delivered.intents.iter().any(|(actor, _, _)| *actor == first));
	assert!(delivered
		.intents
		.iter()
		.any(|(actor, _, intent)| *actor == second && *intent == CharacterIntent::Jump));
	assert!(app
		.world()
		.get::<ActorControlStatus>(first)
		.is_some_and(|status| status.is_captured()));
	assert!(app
		.world()
		.get::<RoutedActorControl>(first)
		.and_then(RoutedActorControl::get)
		.is_none());
	assert!(app
		.world()
		.get::<maybraid_character_controller::CharacterIntentFrame>(first)
		.and_then(maybraid_character_controller::CharacterIntentFrame::get)
		.is_none());
	Ok(())
}

fn suspend_on_menu(
	event: On<ActorIntentEvent>,
	mut routing: maybraid_controllers_core::InputRouting,
) {
	if matches!(event.input.payload(), CharacterIntent::InGameMenu) {
		routing.suspend(event.input.source(), ScopeOwner::pause());
	}
}

#[test]
fn shell_pause_retires_same_frame_fire_before_control_delivery() -> Result<()> {
	let mut app = app();
	let first = app.world_mut().spawn_empty().id();
	let second = app.world_mut().spawn_empty().id();
	let first_source = source(&mut app, first);
	let second_source = source(&mut app, second);
	app.add_observer(suspend_on_menu);
	frame(&mut app, first_source, 1, pad(Vec2::X, &[PadButton::Start, PadButton::TriggerFire]));
	frame(&mut app, second_source, 1, pad(Vec2::Y, &[PadButton::TriggerFire]));
	app.update();
	let delivered = app.world().resource::<Delivered>();
	let first_intents: Vec<_> = delivered
		.intents
		.iter()
		.filter(|(actor, _, _)| *actor == first)
		.map(|(_, _, intent)| *intent)
		.collect();
	assert_eq!(first_intents, [CharacterIntent::InGameMenu]);
	assert!(!delivered.controls.iter().any(|(actor, _, _)| *actor == first));
	assert!(delivered
		.controls
		.iter()
		.any(|(actor, _, sample)| *actor == second && sample.fire_held()));
	assert!(app
		.world()
		.get::<ActorControlStatus>(first)
		.is_some_and(|status| status.is_frozen()));
	Ok(())
}
