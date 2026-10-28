//! End-to-end jab lifetime: routed pad → handler → sensor → hit / teardown.

use std::sync::{Mutex, OnceLock};

use avian3d::prelude::*;
use bevy::ecs::error::{BevyError, ErrorContext};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use damage::{DamageApplied, Health, Hit};
use lod_avian::PhysicsInteractionLayer;
use maybraid_character_controller::{CharacterControllerPlugin, OrdinaryLocomotion};
use maybraid_controllers_core::{ActorTarget, InputRouter, InputSource};
use maybraid_equipment::{
	EquipmentPlugin, EquippedItemHandler, PendingEquipmentHold, PendingEquipmentTeardown,
};
use maybraid_virtual_pad_controller::{PadOverlay, PadOverlayRequest, PrimaryVirtualPad};
use player::{Player, PlayerLook};
use std::time::Duration;

use crate::{
	AttackPhase, AttackTiming, FisticuffsHitSensor, FisticuffsUser, FisticuffsUserPlugin,
	JAB_STARTUP_END, PLACEHOLDER_JAB_DAMAGE,
};

// The Bevy fallback handler is a function pointer, so command errors use a
// process-wide collector. Hold this separate lock for each complete test to
// prevent another app from clearing or consuming its errors mid-frame.
fn lock_lifetime_test() -> std::sync::MutexGuard<'static, ()> {
	static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
	TEST_LOCK
		.get_or_init(|| Mutex::new(()))
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn command_errors() -> &'static Mutex<Vec<String>> {
	static ERRORS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
	ERRORS.get_or_init(|| Mutex::new(Vec::new()))
}

fn lock_command_errors() -> std::sync::MutexGuard<'static, Vec<String>> {
	command_errors().lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn record_command_error(error: BevyError, ctx: ErrorContext) {
	lock_command_errors().push(format!("{} {}: {error}", ctx.kind(), ctx.name()));
}

fn clear_command_errors() {
	lock_command_errors().clear();
}

fn assert_no_command_errors() -> anyhow::Result<()> {
	let errors = lock_command_errors().clone();
	if errors.iter().any(|error| {
		error.contains("despawned") || error.contains("invalid") || error.contains("generation")
	}) {
		anyhow::bail!("despawned-entity command error: {errors:?}");
	}
	if !errors.is_empty() {
		anyhow::bail!("command / system errors (recovery would escalate): {errors:?}");
	}
	Ok(())
}

fn lifetime_app() -> App {
	lifetime_app_with_presentation(true)
}

fn lifetime_app_with_presentation(presentation: bool) -> App {
	clear_command_errors();
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		bevy::input::InputPlugin,
		PhysicsPlugins::default(),
		bevy::asset::AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
		CharacterControllerPlugin,
		EquipmentPlugin,
	));
	if presentation {
		app.add_plugins(FisticuffsUserPlugin);
	} else {
		crate::register_fisticuffs_user_runtime(&mut app);
	}
	app.finish();
	app.set_error_handler(record_command_error);
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(1.0 / 60.0)));
	app.add_message::<Hit>().add_message::<DamageApplied>();
	app
}

fn assign(app: &mut App, actor: Entity) {
	app.world_mut()
		.resource_mut::<InputRouter>()
		.assign_actor(InputSource::local_pad(), ActorTarget::new(actor));
}

fn squeeze_right_trigger(app: &mut App) {
	let source = app.world().resource::<PrimaryVirtualPad>().0;
	app.world_mut().write_message(PadOverlayRequest {
		source,
		overlay: Some(PadOverlay { trigger_fire: 0.9, ..default() }),
	});
}

fn release_triggers(app: &mut App) {
	let source = app.world().resource::<PrimaryVirtualPad>().0;
	app.world_mut()
		.write_message(PadOverlayRequest { source, overlay: Some(PadOverlay::default()) });
}

fn sensor_count(world: &mut World) -> usize {
	world.query::<&FisticuffsHitSensor>().iter(world).count()
}

fn spawn_attacker(app: &mut App) -> Entity {
	app.world_mut()
		.spawn((
			Player,
			FisticuffsUser::default(),
			EquippedItemHandler::Fisticuffs,
			OrdinaryLocomotion(true),
			PlayerLook::default(),
			Health::from_max(100.0),
			Transform::from_translation(Vec3::ZERO),
			GlobalTransform::from_translation(Vec3::ZERO),
		))
		.id()
}

fn spawn_dummy(app: &mut App) -> Entity {
	app.world_mut()
		.spawn((
			Health::from_max(100.0),
			Transform::from_translation(Vec3::new(0.16, 0.62, -1.28)),
			GlobalTransform::from_translation(Vec3::new(0.16, 0.62, -1.28)),
			RigidBody::Kinematic,
			Collider::capsule(0.4, 1.0),
			PhysicsInteractionLayer::animated_layers(),
		))
		.id()
}

fn advance_to_active(app: &mut App) {
	let frames =
		((AttackTiming::JAB.duration_secs * JAB_STARTUP_END) / (1.0 / 60.0)).ceil() as u32 + 2;
	for _ in 0..frames {
		app.update();
	}
}

#[test]
fn routed_jab_hits_then_teardown_paths_do_not_queue_despawned_inserts() -> anyhow::Result<()> {
	let _test_guard = lock_lifetime_test();
	let mut app = lifetime_app();
	let player = spawn_attacker(&mut app);
	let dummy = spawn_dummy(&mut app);
	assign(&mut app, player);

	squeeze_right_trigger(&mut app);
	app.update();
	release_triggers(&mut app);
	assert_no_command_errors()?;
	{
		let user = app.world().get::<FisticuffsUser>(player).ok_or(anyhow::anyhow!("user"))?;
		if !user.state.is_busy() {
			anyhow::bail!("routed AttackRight must chamber a jab");
		}
	}

	advance_to_active(&mut app);
	assert_no_command_errors()?;
	if sensor_count(app.world_mut()) != 1 {
		anyhow::bail!("Active window should spawn one pose-only fist sensor");
	}

	app.update();
	app.update();
	assert_no_command_errors()?;
	let remaining = app.world().get::<Health>(dummy).map(|health| health.current);
	if remaining != Some(100.0 - PLACEHOLDER_JAB_DAMAGE) {
		anyhow::bail!("routed jab should land placeholder damage, got {remaining:?}");
	}

	for _ in 0..30 {
		app.update();
	}
	assert_no_command_errors()?;
	if sensor_count(app.world_mut()) != 0 {
		anyhow::bail!("recovery must expire the fist sensor");
	}
	if app.world().get::<FisticuffsUser>(player).is_none() {
		anyhow::bail!("natural expire must keep the driver");
	}

	// Swap / unequip mid-jab: chamber again, then request equipment teardown.
	squeeze_right_trigger(&mut app);
	app.update();
	release_triggers(&mut app);
	advance_to_active(&mut app);
	if sensor_count(app.world_mut()) != 1 {
		anyhow::bail!("second jab should spawn a sensor before swap");
	}
	app.world_mut().entity_mut(player).insert(PendingEquipmentTeardown);
	app.update();
	assert_no_command_errors()?;
	if app.world().get::<FisticuffsUser>(player).is_some() {
		anyhow::bail!("equipment teardown must drop FisticuffsUser");
	}
	if sensor_count(app.world_mut()) != 0 {
		anyhow::bail!("swap mid-jab must tear the sensor down");
	}

	// Re-hold, then die mid-jab.
	app.world_mut().entity_mut(player).insert((
		FisticuffsUser::default(),
		EquippedItemHandler::Fisticuffs,
		PendingEquipmentHold::new(character_items::Inventory {
			items: vec![character_items::InventoryItem::Fisticuffs],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		}),
	));
	app.world_mut().entity_mut(player).remove::<PendingEquipmentTeardown>();
	app.update();
	squeeze_right_trigger(&mut app);
	app.update();
	release_triggers(&mut app);
	advance_to_active(&mut app);
	if sensor_count(app.world_mut()) != 1 {
		anyhow::bail!("re-equipped jab should spawn a sensor before death");
	}
	if let Some(mut health) = app.world_mut().get_mut::<Health>(player) {
		health.current = 0.0;
	}
	app.update();
	assert_no_command_errors()?;
	if sensor_count(app.world_mut()) != 0 {
		anyhow::bail!("attacker death mid-jab must tear the sensor down");
	}
	Ok(())
}

#[test]
fn captured_pad_does_not_chamber_a_jab() -> anyhow::Result<()> {
	let _test_guard = lock_lifetime_test();
	use maybraid_controllers_core::{CaptureKind, ScopeOwner, UiTarget};

	let mut app = lifetime_app();
	let player = spawn_attacker(&mut app);
	assign(&mut app, player);
	{
		let mut router = app.world_mut().resource_mut::<InputRouter>();
		router.acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::ui(UiTarget::new(Entity::from_bits(9))),
			CaptureKind::Overlay,
		);
	}
	squeeze_right_trigger(&mut app);
	app.update();
	assert_no_command_errors()?;
	let user = app.world().get::<FisticuffsUser>(player).ok_or(anyhow::anyhow!("user"))?;
	if user.state.is_busy() {
		anyhow::bail!("captured pad must not chamber a jab");
	}
	if user.state.phase() == Some(AttackPhase::Active) {
		anyhow::bail!("captured pad must not enter Active");
	}
	Ok(())
}

#[test]
fn runtime_only_jab_reaches_damage_without_the_presentation_plugin() -> anyhow::Result<()> {
	let _test_guard = lock_lifetime_test();
	let mut app = lifetime_app_with_presentation(false);
	let player = spawn_attacker(&mut app);
	let dummy = spawn_dummy(&mut app);
	assign(&mut app, player);
	squeeze_right_trigger(&mut app);
	app.update();
	release_triggers(&mut app);
	advance_to_active(&mut app);
	app.update();
	let health = app
		.world()
		.get::<Health>(dummy)
		.ok_or_else(|| anyhow::anyhow!("dummy health"))?;
	assert_eq!(health.current, health.max - PLACEHOLDER_JAB_DAMAGE);
	assert!(!app.is_plugin_added::<FisticuffsUserPlugin>());
	assert!(!app.is_plugin_added::<maybraid_audio::AudioPlugin>());
	assert_no_command_errors()?;
	Ok(())
}

#[test]
fn routing_loss_cancels_only_the_previous_actor_before_damage() -> anyhow::Result<()> {
	let _test_guard = lock_lifetime_test();
	use maybraid_controllers_core::{CaptureKind, InputRouting, ScopeOwner, UiTarget};

	for transition in ["capture", "disconnect", "reassign"] {
		let mut app = lifetime_app_with_presentation(false);
		let actor = spawn_attacker(&mut app);
		let dummy = spawn_dummy(&mut app);
		app.world_mut().entity_mut(dummy).remove::<Health>();
		assign(&mut app, actor);
		squeeze_right_trigger(&mut app);
		app.update();
		release_triggers(&mut app);
		advance_to_active(&mut app);
		app.world_mut().entity_mut(dummy).insert(Health::from_max(100.0));
		let mut other_user = FisticuffsUser::default();
		other_user.state.request(crate::AttackSpec::jab(crate::Side::Left));
		let other = app.world_mut().spawn((other_user, Transform::from_xyz(100.0, 0.0, 0.0))).id();
		app.world_mut()
			.run_system_once(move |mut routing: InputRouting| match transition {
				"capture" => {
					routing.acquire_capture(
						InputSource::local_pad(),
						ScopeOwner::ui(UiTarget::new(dummy)),
						CaptureKind::Overlay,
					);
				}
				"disconnect" => {
					routing.disconnect(InputSource::local_pad());
				}
				_ => {
					routing.assign_actor(InputSource::local_pad(), ActorTarget::new(other));
				}
			})
			.map_err(|error| anyhow::anyhow!("{error}"))?;
		app.update();
		let user = app
			.world()
			.get::<FisticuffsUser>(actor)
			.ok_or_else(|| anyhow::anyhow!("actor"))?;
		assert!(!user.state.is_busy(), "{transition} must cancel old actor");
		let other_user = app
			.world()
			.get::<FisticuffsUser>(other)
			.ok_or_else(|| anyhow::anyhow!("other"))?;
		assert!(other_user.state.is_busy(), "{transition} must leave other actors alone");
		let health = app.world().get::<Health>(dummy).ok_or_else(|| anyhow::anyhow!("dummy"))?;
		assert_eq!(health.current, health.max, "{transition} must precede hit resolution");
		assert_eq!(sensor_count(app.world_mut()), 0);
		assert_no_command_errors()?;
	}
	Ok(())
}

#[test]
fn suspension_freezes_local_attack_and_damage_while_other_actors_continue() -> anyhow::Result<()> {
	let _test_guard = lock_lifetime_test();
	use maybraid_controllers_core::{InputRouting, ScopeOwner};

	let mut app = lifetime_app_with_presentation(false);
	let actor = spawn_attacker(&mut app);
	let dummy = spawn_dummy(&mut app);
	// Keep the target out of physics until the actor is suspended. A solid
	// without Health still consumes a jab as an environmental contact.
	app.world_mut().entity_mut(dummy).remove::<(Health, Collider)>();
	assign(&mut app, actor);
	squeeze_right_trigger(&mut app);
	app.update();
	release_triggers(&mut app);
	advance_to_active(&mut app);
	let progress = app
		.world()
		.get::<FisticuffsUser>(actor)
		.ok_or_else(|| anyhow::anyhow!("actor"))?
		.state
		.clip_progress();
	assert_eq!(
		app.world().get::<FisticuffsUser>(actor).map(|user| user.state.phase()),
		Some(Some(AttackPhase::Active))
	);
	app.world_mut()
		.entity_mut(dummy)
		.insert((Health::from_max(100.0), Collider::capsule(0.4, 1.0)));
	let mut other_user = FisticuffsUser::default();
	other_user.state.request(crate::AttackSpec::jab(crate::Side::Left));
	let other = app.world_mut().spawn((other_user, Transform::from_xyz(100.0, 0.0, 0.0))).id();
	app.world_mut()
		.run_system_once(|mut routing: InputRouting| {
			routing.suspend(InputSource::local_pad(), ScopeOwner::pause());
		})
		.map_err(|error| anyhow::anyhow!("{error}"))?;
	for _ in 0..6 {
		app.update();
	}
	let user = app
		.world()
		.get::<FisticuffsUser>(actor)
		.ok_or_else(|| anyhow::anyhow!("actor"))?;
	assert_eq!(user.state.clip_progress(), progress);
	let other_user = app
		.world()
		.get::<FisticuffsUser>(other)
		.ok_or_else(|| anyhow::anyhow!("other"))?;
	assert!(other_user.state.clip_progress().is_some_and(|progress| progress > 0.0));
	let health = app.world().get::<Health>(dummy).ok_or_else(|| anyhow::anyhow!("dummy"))?;
	assert_eq!(health.current, health.max, "a frozen active sensor cannot deal damage");
	app.world_mut()
		.run_system_once(|mut routing: InputRouting| {
			routing.resume_owner(ScopeOwner::pause());
		})
		.map_err(|error| anyhow::anyhow!("{error}"))?;
	app.update();
	app.update();
	let health = app.world().get::<Health>(dummy).ok_or_else(|| anyhow::anyhow!("dummy"))?;
	assert_eq!(
		health.current,
		health.max - PLACEHOLDER_JAB_DAMAGE,
		"resume deals one hit, without replaying the active window"
	);
	assert_no_command_errors()?;
	Ok(())
}

#[derive(Resource, Default)]
struct HitchStrikes(usize);

fn count_hitch_strikes(
	mut strikes: MessageReader<crate::FisticuffsStrike>,
	mut count: ResMut<HitchStrikes>,
) {
	count.0 += strikes.read().count();
}

#[test]
fn a_hitch_across_the_hit_window_still_emits_one_strike_and_deals_one_hit() -> anyhow::Result<()> {
	let _test_guard = lock_lifetime_test();
	let mut app = lifetime_app_with_presentation(false);
	app.init_resource::<HitchStrikes>().add_systems(PostUpdate, count_hitch_strikes);
	let actor = spawn_attacker(&mut app);
	let dummy = spawn_dummy(&mut app);
	assign(&mut app, actor);
	app.update();
	squeeze_right_trigger(&mut app);
	app.update();
	release_triggers(&mut app);
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(130)));
	app.update();
	assert_eq!(
		app.world().get::<FisticuffsUser>(actor).and_then(|user| user.state.phase()),
		Some(AttackPhase::Startup)
	);
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(100)));
	app.update();
	assert_eq!(
		app.world().get::<FisticuffsUser>(actor).and_then(|user| user.state.phase()),
		Some(AttackPhase::Active)
	);
	assert_eq!(app.world().resource::<HitchStrikes>().0, 1);
	assert_eq!(sensor_count(app.world_mut()), 1);
	let health = app
		.world()
		.get::<Health>(dummy)
		.ok_or_else(|| anyhow::anyhow!("dummy health"))?;
	assert_eq!(health.current, health.max - PLACEHOLDER_JAB_DAMAGE);
	for _ in 0..5 {
		app.update();
	}
	assert_eq!(sensor_count(app.world_mut()), 0);
	assert_eq!(app.world().resource::<HitchStrikes>().0, 1);
	let health = app
		.world()
		.get::<Health>(dummy)
		.ok_or_else(|| anyhow::anyhow!("dummy health"))?;
	assert_eq!(health.current, health.max - PLACEHOLDER_JAB_DAMAGE);
	assert_no_command_errors()?;
	Ok(())
}
