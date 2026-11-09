//! Real-physics regressions for contact-loss grace and unsupported capsules.

use super::*;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Resource, Default)]
struct MissNextGroundProbe(bool);

#[derive(Resource, Default)]
struct SteepenGroundProbes(bool);

fn miss_one_probe(
	mut miss: ResMut<MissNextGroundProbe>,
	steepen: Res<SteepenGroundProbes>,
	mut hits: Query<&mut ShapeHits>,
) {
	if std::mem::take(&mut miss.0) {
		for mut hits in &mut hits {
			hits.0.clear();
		}
	}
	if steepen.0 {
		for mut hits in &mut hits {
			for hit in &mut hits.0 {
				hit.normal2 = Vec3::X;
			}
		}
	}
}

fn physics_app(hz: u32) -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		bevy::asset::AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
	));
	crate::contact::register_motor_traction_physics(&mut app);
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
		1.0 / f64::from(hz),
	)))
	.init_resource::<MissNextGroundProbe>()
	.init_resource::<SteepenGroundProbes>()
	.add_systems(Update, (miss_one_probe, update_grounded, apply_wish_movement).chain());
	app.finish();
	app
}

fn floor(app: &mut App, width: f32) -> Entity {
	app.world_mut()
		.spawn((
			Transform::from_xyz(0.0, -0.1, 0.0),
			RigidBody::Static,
			Collider::cuboid(width, 0.2, 20.0),
			PhysicsInteractionLayer::fixed_layers(),
		))
		.id()
}

fn settled_controller(app: &mut App, floor: Entity) -> anyhow::Result<Entity> {
	let hull = LocomotionCapsule::HUMANOID;
	let entity = {
		let mut commands = app.world_mut().commands();
		let entity = spawn_character_controller(&mut commands, Vec3::Y * hull.half_height(), hull);
		// Isolate contact-loss behavior from solver sleep/wake heuristics.
		commands.entity(entity).insert(SleepingDisabled);
		entity
	};
	app.world_mut().flush();
	for _ in 0..64 {
		app.update();
	}
	anyhow::ensure!(app.world().get::<Grounded>(entity).is_some(), "fixture never grounded");
	anyhow::ensure!(
		app.world().get::<ShapeHits>(entity).is_some_and(|hits| !hits.is_empty()),
		"fixture has no real downward shape contact"
	);
	anyhow::ensure!(
		app.world()
			.resource::<ContactGraph>()
			.get(entity, floor)
			.is_some_and(|(_, pair)| pair.is_touching() && pair.generates_constraints()),
		"fixture never reached real solver contact"
	);
	Ok(entity)
}

fn position(app: &App, entity: Entity) -> anyhow::Result<Vec3> {
	app.world()
		.get::<Position>(entity)
		.map(|position| position.0)
		.ok_or_else(|| anyhow::anyhow!("controller position missing"))
}

#[test]
fn removed_floor_releases_grounded_and_restores_free_fall() -> anyhow::Result<()> {
	for hz in [30, 64, 120] {
		removed_floor_at_rate(hz)?;
	}
	Ok(())
}

fn removed_floor_at_rate(hz: u32) -> anyhow::Result<()> {
	let mut app = physics_app(hz);
	let floor = floor(&mut app, 20.0);
	let entity = settled_controller(&mut app, floor)?;
	let before = position(&app, entity)?;
	anyhow::ensure!(app.world_mut().despawn(floor), "fixture floor vanished");
	for _ in 0..hz.div_ceil(4) {
		app.update();
	}
	assert!(
		app.world().get::<Grounded>(entity).is_none(),
		"a sustained probe miss must end ground grace within about 0.25 s at {hz} Hz"
	);
	for _ in 0..(hz - hz.div_ceil(4)) {
		app.update();
	}
	assert!(
		position(&app, entity)?.y < before.y - 1.0,
		"an idle capsule must fall after support disappears at {hz} Hz"
	);
	Ok(())
}

#[test]
fn walking_off_ledge_releases_grounded_and_falls() -> anyhow::Result<()> {
	for hz in [30, 64, 120] {
		walking_off_ledge_at_rate(hz)?;
	}
	Ok(())
}

fn walking_off_ledge_at_rate(hz: u32) -> anyhow::Result<()> {
	let mut app = physics_app(hz);
	let floor = floor(&mut app, 4.0);
	let entity = settled_controller(&mut app, floor)?;
	let before = position(&app, entity)?;
	app.world_mut()
		.get_mut::<MoveWish>(entity)
		.ok_or_else(|| anyhow::anyhow!("controller wish missing"))?
		.0 = Vec3::X;
	for _ in 0..hz * 2 {
		app.update();
	}
	let after = position(&app, entity)?;
	assert!(after.x > 4.0, "fixture did not walk beyond the ledge: {after:?}");
	assert!(app.world().get::<Grounded>(entity).is_none(), "ledge departure stayed grounded");
	assert!(after.y < before.y - 1.0, "ledge departure must fall: {before:?} -> {after:?}");
	Ok(())
}

#[test]
fn isolated_probe_misses_keep_ground_grace_and_recover() -> anyhow::Result<()> {
	for hz in [30, 64, 120] {
		isolated_probe_misses_at_rate(hz)?;
	}
	Ok(())
}

fn isolated_probe_misses_at_rate(hz: u32) -> anyhow::Result<()> {
	let mut app = physics_app(hz);
	let floor = floor(&mut app, 20.0);
	let entity = settled_controller(&mut app, floor)?;
	let before = position(&app, entity)?;
	// A single missing sample represents a triangle-edge or streaming seam.
	// Fresh contacts between misses must reset grace, rather than accumulating.
	for _ in 0..8 {
		app.world_mut().resource_mut::<MissNextGroundProbe>().0 = true;
		app.update();
		assert!(app.world().get::<Grounded>(entity).is_some(), "one miss lost ground grace");
		for _ in 0..4 {
			app.update();
		}
		anyhow::ensure!(
			app.world().get::<ShapeHits>(entity).is_some_and(|hits| !hits.is_empty()),
			"real physics did not restore the contact sample"
		);
		assert!(app.world().get::<Grounded>(entity).is_some());
	}
	assert!((position(&app, entity)?.y - before.y).abs() < 0.1);
	Ok(())
}

#[test]
fn airborne_jump_does_not_reuse_ground_grace() -> anyhow::Result<()> {
	let mut app = physics_app(64);
	let floor = floor(&mut app, 20.0);
	let entity = settled_controller(&mut app, floor)?;
	let mut jumping = Jumping::start(0.0);
	jumping.phase = JumpPhase::Air;
	app.world_mut().entity_mut(entity).insert(jumping);
	app.world_mut().resource_mut::<MissNextGroundProbe>().0 = true;
	app.update();
	assert!(app.world().get::<Grounded>(entity).is_none());
	anyhow::ensure!(
		app.world().get::<Jumping>(entity).is_some_and(|jump| jump.left_ground),
		"airborne jump must observe contact loss"
	);
	Ok(())
}

#[test]
fn steep_contacts_do_not_replenish_ground_grace() -> anyhow::Result<()> {
	let mut app = physics_app(64);
	let floor = floor(&mut app, 20.0);
	let entity = settled_controller(&mut app, floor)?;
	// Retain real hit distances/entities while making every reported surface
	// unwalkable. Actual solver contacts still support the fixture body.
	app.world_mut().resource_mut::<SteepenGroundProbes>().0 = true;
	app.update();
	assert!(app.world().get::<Grounded>(entity).is_some(), "first miss should retain grace");
	for _ in 0..16 {
		app.update();
	}
	assert!(app.world().get::<Grounded>(entity).is_none(), "steep hits renewed walkable grace");
	Ok(())
}

#[test]
fn grounded_marker_without_a_contact_does_not_grant_ground_grace() {
	let mut app = physics_app(64);
	let entity = {
		let mut commands = app.world_mut().commands();
		let entity =
			spawn_character_controller(&mut commands, Vec3::Y * 10.0, LocomotionCapsule::HUMANOID);
		commands.entity(entity).insert((Grounded, SleepingDisabled));
		entity
	};
	app.world_mut().flush();
	// Allow physics to initialize ShapeHits; no floor has ever existed.
	for _ in 0..2 {
		app.update();
	}
	assert!(
		app.world().get::<Grounded>(entity).is_none(),
		"a marker alone must not create remembered support"
	);
}
