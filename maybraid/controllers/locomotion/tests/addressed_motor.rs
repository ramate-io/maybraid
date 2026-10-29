use std::time::Duration;

use anyhow::{anyhow, Result};
use bevy::prelude::*;
use maybraid_character_controller::{
	register_character_controller, ActorControlStatus, OrdinaryLocomotion,
};
use maybraid_controllers_core::{
	ActorTarget, CaptureKind, ControllerPipeline, InputRouter, InputRouting, InputSource,
	ScopeOwner, UiTarget,
};
use maybraid_locomotion_controller::interpret_standard_locomotion;
use maybraid_virtual_pad_controller::{
	PadButton, PadDevices, VirtualPad, VirtualPadController, VirtualPadEvent,
};
use player::{
	apply_motor, apply_routing_hold_effects, compose_motor, CharacterController, MotorIntent,
	MotorRequestStage, MotorSelection, MoveWish, Player, PlayerLook, ResolvedMotorRequest,
	Sprinting,
};

fn reset(mut actors: Query<&mut MotorIntent>) {
	for mut motor in &mut actors {
		motor.clear();
	}
}

fn app() -> App {
	let mut app = App::new();
	register_character_controller(&mut app);
	MotorRequestStage::configure(&mut app);
	app.add_systems(
		Update,
		(
			reset.in_set(ControllerPipeline::ResetMotorRequests),
			interpret_standard_locomotion.in_set(MotorRequestStage::Standard),
			apply_routing_hold_effects
				.in_set(ControllerPipeline::MotorRequest)
				.after(MotorRequestStage::WorldFilter),
			compose_motor.in_set(ControllerPipeline::Motor),
			apply_motor.in_set(ControllerPipeline::Motor).after(compose_motor),
		),
	);
	app
}

fn actor(app: &mut App) -> Entity {
	app.world_mut()
		.spawn((
			CharacterController,
			Player,
			MoveWish::default(),
			MotorIntent::default(),
			MotorSelection::default(),
			ResolvedMotorRequest::default(),
			PlayerLook::default(),
		))
		.id()
}

fn source(app: &mut App, actor: Entity) -> Entity {
	let source = app.world_mut().spawn(VirtualPadController::new(PadDevices::default())).id();
	app.world_mut()
		.resource_mut::<InputRouter>()
		.assign_actor(InputSource::virtual_pad(source), ActorTarget::new(actor));
	source
}

fn frame(app: &mut App, source: Entity, sequence: u64, stick: Vec2, buttons: &[PadButton]) {
	let mut pad = VirtualPad::default();
	pad.begin_frame();
	pad.add_move(stick);
	for button in buttons {
		pad.hold_digital(*button);
	}
	pad.finish_digital();
	app.world_mut().write_message(VirtualPadEvent {
		source,
		sequence,
		delta: Duration::from_millis(16),
		pad,
	});
}

fn wish(app: &App, actor: Entity) -> Result<Vec3> {
	app.world()
		.get::<MoveWish>(actor)
		.map(|wish| wish.0)
		.ok_or_else(|| anyhow!("actor wish"))
}

#[test]
fn two_actors_move_hold_and_release_independently() -> Result<()> {
	let mut app = app();
	let first = actor(&mut app);
	let second = actor(&mut app);
	let first_source = source(&mut app, first);
	let second_source = source(&mut app, second);
	frame(&mut app, first_source, 1, Vec2::Y, &[PadButton::StickClickMove]);
	frame(&mut app, second_source, 1, Vec2::X, &[]);
	app.update();
	assert_eq!(wish(&app, first)?, -Vec3::Z);
	assert_eq!(wish(&app, second)?, Vec3::X);
	assert!(app.world().get::<Sprinting>(first).is_some());
	assert!(app.world().get::<Sprinting>(second).is_none());
	frame(&mut app, first_source, 2, Vec2::ZERO, &[]);
	frame(&mut app, second_source, 2, Vec2::NEG_X, &[PadButton::StickClickMove]);
	app.update();
	assert_eq!(wish(&app, first)?, Vec3::ZERO);
	assert_eq!(wish(&app, second)?, -Vec3::X);
	assert!(app.world().get::<Sprinting>(second).is_some());
	// Absence retires the first actor's old movement and holds on the motor apply path.
	app.update();
	assert_eq!(wish(&app, first)?, Vec3::ZERO);
	assert_eq!(wish(&app, second)?, Vec3::ZERO);
	assert!(app.world().get::<Sprinting>(second).is_none());
	Ok(())
}

#[test]
fn capture_cancels_one_actor_and_suspension_preserves_its_holds() -> Result<()> {
	let mut app = app();
	let first = actor(&mut app);
	let second = actor(&mut app);
	let first_source = source(&mut app, first);
	let second_source = source(&mut app, second);
	frame(&mut app, first_source, 1, Vec2::Y, &[PadButton::StickClickMove]);
	frame(&mut app, second_source, 1, Vec2::X, &[PadButton::StickClickMove]);
	app.update();
	let suspension = app
		.world_mut()
		.resource_mut::<InputRouter>()
		.suspend(InputSource::virtual_pad(first_source), ScopeOwner::pause());
	for sequence in 2..5 {
		frame(&mut app, second_source, sequence, Vec2::X, &[PadButton::StickClickMove]);
		app.update();
		assert_eq!(wish(&app, first)?, Vec3::ZERO);
		assert!(app.world().get::<Sprinting>(first).is_some());
		assert_eq!(wish(&app, second)?, Vec3::X);
	}
	app.world_mut().resource_mut::<InputRouter>().resume(suspension);
	let menu = app.world_mut().spawn_empty().id();
	app.world_mut().resource_mut::<InputRouter>().acquire_capture(
		InputSource::virtual_pad(first_source),
		ScopeOwner::ui(UiTarget::new(menu)),
		CaptureKind::MenuNav,
	);
	frame(&mut app, first_source, 5, Vec2::Y, &[PadButton::StickClickMove]);
	frame(&mut app, second_source, 5, Vec2::X, &[PadButton::StickClickMove]);
	app.update();
	assert_eq!(wish(&app, first)?, Vec3::ZERO);
	assert!(app.world().get::<Sprinting>(first).is_none());
	assert_eq!(wish(&app, second)?, Vec3::X);
	assert!(app.world().get::<Sprinting>(second).is_some());
	Ok(())
}

#[derive(Resource)]
struct Reassign {
	source: InputSource,
	actor: ActorTarget,
}

fn reassign_once(change: Option<Res<Reassign>>, mut routing: InputRouting, mut commands: Commands) {
	if let Some(change) = change {
		routing.assign_actor(change.source, change.actor);
		commands.remove_resource::<Reassign>();
	}
}

#[test]
fn reassignment_after_bind_cannot_apply_the_old_frame_to_either_actor() -> Result<()> {
	let mut app = app();
	let first = actor(&mut app);
	let second = actor(&mut app);
	let source = source(&mut app, first);
	app.insert_resource(Reassign {
		source: InputSource::virtual_pad(source),
		actor: ActorTarget::new(second),
	})
	.add_systems(
		Update,
		reassign_once
			.after(ControllerPipeline::Bind)
			.before(ControllerPipeline::ResetMotorRequests),
	);
	frame(&mut app, source, 1, Vec2::Y, &[PadButton::StickClickMove]);
	app.update();
	assert_eq!(wish(&app, first)?, Vec3::ZERO);
	assert_eq!(wish(&app, second)?, Vec3::ZERO);
	assert!(app.world().get::<Sprinting>(first).is_none());
	assert!(app.world().get::<Sprinting>(second).is_none());
	assert!(app
		.world()
		.get::<ActorControlStatus>(first)
		.is_some_and(|status| !status.is_controlled()));
	assert_eq!(
		app.world().get::<ActorControlStatus>(second).and_then(|status| status.source()),
		Some(InputSource::virtual_pad(source))
	);
	frame(&mut app, source, 2, Vec2::X, &[]);
	app.update();
	assert_eq!(wish(&app, first)?, Vec3::ZERO);
	assert_eq!(
		wish(&app, second)?,
		Vec3::ZERO,
		"changing direction without centering must not transfer a held stick"
	);
	frame(&mut app, source, 3, Vec2::ZERO, &[]);
	app.update();
	assert_eq!(wish(&app, first)?, Vec3::ZERO);
	assert_eq!(wish(&app, second)?, Vec3::ZERO);
	frame(&mut app, source, 4, Vec2::X, &[]);
	app.update();
	assert_eq!(wish(&app, first)?, Vec3::ZERO);
	assert_eq!(wish(&app, second)?, Vec3::X);
	Ok(())
}

#[test]
fn tanzer_mode_and_focus_are_actor_local() -> Result<()> {
	let mut app = app();
	let first = actor(&mut app);
	let second = actor(&mut app);
	app.world_mut().entity_mut(first).insert(OrdinaryLocomotion(false));
	let first_source = source(&mut app, first);
	let second_source = source(&mut app, second);
	frame(&mut app, first_source, 1, Vec2::Y, &[PadButton::StickClickMove, PadButton::A]);
	frame(&mut app, second_source, 1, Vec2::X, &[PadButton::StickClickMove]);
	app.update();
	assert_eq!(wish(&app, first)?, Vec3::ZERO);
	assert_eq!(wish(&app, second)?, Vec3::X);
	assert!(app.world().get::<Sprinting>(first).is_none());
	assert!(app.world().get::<Sprinting>(second).is_some());
	let mut pad = VirtualPad::default();
	pad.begin_frame();
	pad.max_triggers(1.0, 0.0);
	pad.hold_digital(PadButton::StickClickMove);
	pad.apply_trigger_digital(
		app.world()
			.get::<VirtualPadController>(second_source)
			.ok_or_else(|| anyhow!("second controller"))?
			.config
			.trigger_press_threshold,
	);
	pad.finish_digital();
	app.world_mut().write_message(VirtualPadEvent {
		source: second_source,
		sequence: 2,
		delta: Duration::from_millis(16),
		pad,
	});
	app.update();
	assert!(app.world().get::<Sprinting>(second).is_none());
	Ok(())
}
