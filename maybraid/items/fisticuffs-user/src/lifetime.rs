//! End-to-end jab lifetime: routed pad → handler → sensor → hit / teardown.

use std::sync::{Mutex, OnceLock};

use avian3d::prelude::*;
use bevy::ecs::error::{BevyError, ErrorContext};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use damage::{DamageApplied, Health, Hit};
use lod_avian::PhysicsInteractionLayer;
use maybraid_character_controller::produce::produce_character_intents;
use maybraid_character_controller::{
	CharacterControllerPlugin, CharacterIntent, OrdinaryLocomotion,
};
use maybraid_controllers_core::{ActorTarget, InputRouter, InputSource};
use maybraid_equipment::{
	EquipmentPlugin, EquippedItemHandler, PendingEquipmentHold, PendingEquipmentTeardown,
};
use maybraid_input::{VirtualPad, VirtualPadConfig};
use player::{Player, PlayerLook};
use std::time::Duration;

use crate::{
	AttackPhase, AttackTiming, FisticuffsHitSensor, FisticuffsUser, FisticuffsUserPlugin,
	JAB_STARTUP_END, PLACEHOLDER_JAB_DAMAGE,
};

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
	{
		let mut pad = app.world_mut().resource_mut::<VirtualPad>();
		pad.begin_frame();
		pad.max_triggers(0.0, 0.9);
		pad.apply_trigger_digital(0.5);
		pad.finish_digital();
	}
	app.world_mut().run_system_once(produce_character_intents).expect("produce jab");
}

fn release_triggers(app: &mut App) {
	{
		let mut pad = app.world_mut().resource_mut::<VirtualPad>();
		pad.begin_frame();
		pad.finish_digital();
	}
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
	let mut app = lifetime_app();
	app.world_mut().insert_resource(VirtualPadConfig::default());
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
			anyhow::bail!("routed AttackRight must chamber a jab, intents={:?}", {
				app.world_mut()
					.resource_mut::<Messages<CharacterIntent>>()
					.drain()
					.collect::<Vec<_>>()
			});
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
	use maybraid_controllers_core::{CaptureKind, ScopeOwner, UiTarget};

	let mut app = lifetime_app();
	app.world_mut().insert_resource(VirtualPadConfig::default());
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
