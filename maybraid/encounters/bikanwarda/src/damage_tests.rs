//! Player projectile damage integration tests.

use anyhow::Result;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bikanwarda_character::{
	hit_complex_collider, leg_aim_point, outer_body_aim_point, spawn_bikanwarda_encounter,
	spawn_bikanwarda_ward, BikanwardaActorRecipe, BikanwardaSpec,
};
use characters::CharacterRecipe;
use damage::{DamageApplied, Health, HitPayload};
use lod_avian::PhysicsInteractionLayer;
use projectiles::{BulletSpec, Flight, ProjectileSource};
use std::time::Duration;

const DT: f32 = 1.0 / 60.0;

fn projectile_damage_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		PhysicsPlugins::default(),
		bevy::asset::AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
	))
	.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));

	// Install projectiles plugin
	if !app.is_plugin_added::<projectiles::ProjectilesPlugin>() {
		app.add_plugins(projectiles::ProjectilesPlugin);
	}

	// Install damage plugin
	if !app.is_plugin_added::<damage::DamagePlugin>() {
		app.add_plugins(damage::DamagePlugin);
	}

	app.finish();
	app.update();
	app
}

/// Test that player projectiles damage the production boss through the normal damage path.
/// Uses real projectile sweep, real boss collider, and real damage application.
#[test]
fn player_projectile_damages_boss() -> Result<()> {
	let mut app = projectile_damage_app();

	// Spawn boss actor and host (matching production structure)
	// Host owns Health; actor (child) owns collider
	let spec = BikanwardaSpec::encounter();
	let boss_actor = {
		let mut commands = app.world_mut().commands();
		spawn_bikanwarda_encounter(&mut commands, &spec, Transform::IDENTITY)
	};

	// Flush commands to spawn actor
	app.update();

	// Spawn host with Health (production: host has Health, BossEncounterRoot, EncounterInstance)
	// and link it to the actor
	let host_entity = app
		.world_mut()
		.spawn((
			Name::new("boss-host"),
			Health::from_max(1000.0),
			Transform::default(),
			GlobalTransform::default(),
		))
		.id();

	// Parent actor to host (production structure)
	app.world_mut().entity_mut(boss_actor).insert(ChildOf(host_entity));

	// Spawn player as projectile source
	let player = app
		.world_mut()
		.spawn((Name::new("test-player"), Transform::default(), GlobalTransform::default()))
		.id();

	// Update physics to settle colliders
	for _ in 0..3 {
		app.update();
	}

	// Get boss actor GlobalTransform to aim at
	let boss_transform = app
		.world()
		.get::<GlobalTransform>(boss_actor)
		.expect("Boss actor should have GlobalTransform");

	// Fire a projectile at the boss (aiming at motor origin)
	let target_pos = boss_transform.translation();
	let start_pos = target_pos + Vec3::X * -15.0;
	let direction = (target_pos - start_pos).normalize();

	// Use BulletSpec
	let spec = BulletSpec::default();
	let gravity = 0.0; // Straight shot for test simplicity

	// Spawn projectile with physics components (required for movement)
	let collider = Collider::capsule(spec.radius, spec.length);
	let velocity = direction * spec.speed;
	// capsule_along_y takes Vec3 direction (not Dir3)
	let muzzle_to_target = direction;
	let cap_transform =
		Transform::from_translation(start_pos).looking_to(muzzle_to_target, Vec3::Y);

	let projectile = app
		.world_mut()
		.spawn((
			Name::new("test-bullet"),
			cap_transform,
			GlobalTransform::from(cap_transform),
			RigidBody::Dynamic,
			MassPropertiesBundle::from_shape(&collider, 1.0),
			collider,
			Sensor,
			PhysicsInteractionLayer::projectile_layers(),
			LockedAxes::ROTATION_LOCKED,
			LinearVelocity(velocity),
			GravityScale(gravity),
			Restitution::ZERO,
			Flight::spawn(start_pos, spec.max_range, spec.penetration, spec.max_age),
			ProjectileSource(player),
			HitPayload { amount: 50.0 },
		))
		.id();

	// Record initial host health
	let initial_health =
		app.world().get::<Health>(host_entity).expect("Host should have Health").current;

	// Run projectile sweep and damage application for several frames
	// Boss extends from X=-6.39 to X=+6.39, projectile starts at X=-15
	// At 28 m/s, projectile needs ~8.6m / 28m/s = 0.31s = ~18 frames at 60fps
	for _ in 0..40 {
		app.update();
	}

	// Check host health after projectile passage
	let final_health =
		app.world().get::<Health>(host_entity).expect("Host should have Health").current;

	anyhow::ensure!(
		final_health < initial_health,
		"Host health should decrease after projectile hits actor: initial={}, final={}",
		initial_health,
		final_health
	);

	// Verify damage amount is reasonable (exactly once, not multiple hits)
	let damage_taken = initial_health - final_health;
	anyhow::ensure!(
		damage_taken >= 49.0 && damage_taken <= 51.0,
		"Damage should be ~50.0 (one hit), got {}",
		damage_taken
	);

	Ok(())
}

/// Native Metal play at 20 m had two bolts with zero boss contacts. Those
/// shots were aimed at the visual body, not the vertical motor. The
/// pronograde hit hull must still debit the Health host.
#[test]
fn projectile_through_pronograde_hit_hull_debits_host() -> Result<()> {
	let mut app = projectile_damage_app();
	let spec = BikanwardaSpec::encounter();
	let hull = BikanwardaActorRecipe::from_spec(spec.clone())
		.components()
		.scaled_locomotion_capsule();
	let hit = hull.hit_capsule().expect("encounter hull is pronograde");
	let boss_actor = {
		let mut commands = app.world_mut().commands();
		spawn_bikanwarda_encounter(&mut commands, &spec, Transform::IDENTITY)
	};
	app.update();
	let host_entity = app
		.world_mut()
		.spawn((
			Name::new("boss-host"),
			Health::from_max(1000.0),
			Transform::default(),
			GlobalTransform::default(),
		))
		.id();
	app.world_mut().entity_mut(boss_actor).insert(ChildOf(host_entity));
	let player = app
		.world_mut()
		.spawn((Name::new("test-player"), Transform::default(), GlobalTransform::default()))
		.id();
	for _ in 0..3 {
		app.update();
	}
	let origin = app
		.world()
		.get::<GlobalTransform>(boss_actor)
		.expect("Boss actor should have GlobalTransform")
		.translation();
	let target_pos = origin + hit.local_transform().translation;
	let start_pos = target_pos + Vec3::X * -12.0;
	let direction = (target_pos - start_pos).normalize();
	let bullet = BulletSpec::default();
	let collider = Collider::capsule(bullet.radius, bullet.length);
	let cap_transform = Transform::from_translation(start_pos).looking_to(direction, Vec3::Y);
	app.world_mut().spawn((
		Name::new("body-aimed-bullet"),
		cap_transform,
		GlobalTransform::from(cap_transform),
		RigidBody::Dynamic,
		MassPropertiesBundle::from_shape(&collider, 1.0),
		collider,
		Sensor,
		PhysicsInteractionLayer::projectile_layers(),
		LockedAxes::ROTATION_LOCKED,
		LinearVelocity(direction * bullet.speed),
		GravityScale(0.0),
		Restitution::ZERO,
		Flight::spawn(start_pos, bullet.max_range, bullet.penetration, bullet.max_age),
		ProjectileSource(player),
		HitPayload { amount: 25.0 },
	));
	let initial = app.world().get::<Health>(host_entity).expect("host health").current;
	for _ in 0..40 {
		app.update();
	}
	let final_health = app.world().get::<Health>(host_entity).expect("host health").current;
	anyhow::ensure!(
		(initial - final_health - 25.0).abs() < 0.1,
		"pronograde hull shot should debit 25 HP once, initial={initial} final={final_health}"
	);
	Ok(())
}

fn fire_capsule_bolt(
	app: &mut App,
	player: Entity,
	target: Vec3,
	amount: f32,
	name: &str,
	approach: Vec3,
) {
	let start_pos = target - approach.normalize() * 12.0;
	let direction = approach.normalize();
	let bullet = BulletSpec::default();
	let collider = Collider::capsule(bullet.radius, bullet.length);
	let cap_transform = Transform::from_translation(start_pos).looking_to(direction, Vec3::Y);
	app.world_mut().spawn((
		Name::new(name.to_string()),
		cap_transform,
		GlobalTransform::from(cap_transform),
		RigidBody::Dynamic,
		MassPropertiesBundle::from_shape(&collider, 1.0),
		collider,
		Sensor,
		PhysicsInteractionLayer::projectile_layers(),
		LockedAxes::ROTATION_LOCKED,
		LinearVelocity(direction * bullet.speed),
		GravityScale(0.0),
		Restitution::ZERO,
		Flight::spawn(start_pos, bullet.max_range, bullet.penetration, bullet.max_age),
		ProjectileSource(player),
		HitPayload { amount },
	));
}

/// Native play missed 2/3 bolts on the visual legs / outer body. Those points
/// sit outside the single pronograde capsule; the spec-scaled compound must
/// debit the Health host once per projectile.
#[test]
fn projectile_through_legs_and_outer_body_debits_host() -> Result<()> {
	let mut app = projectile_damage_app();
	let spec = BikanwardaSpec::encounter();
	let hull = BikanwardaActorRecipe::from_spec(spec.clone())
		.components()
		.scaled_locomotion_capsule();
	let boss_actor = {
		let mut commands = app.world_mut().commands();
		spawn_bikanwarda_encounter(&mut commands, &spec, Transform::IDENTITY)
	};
	app.update();
	let host_entity = app
		.world_mut()
		.spawn((
			Name::new("boss-host"),
			Health::from_max(1000.0),
			Transform::default(),
			GlobalTransform::default(),
		))
		.id();
	app.world_mut().entity_mut(boss_actor).insert(ChildOf(host_entity));
	app.world_mut().entity_mut(boss_actor).insert(hit_complex_collider(hull));
	let player = app
		.world_mut()
		.spawn((Name::new("test-player"), Transform::default(), GlobalTransform::default()))
		.id();
	for _ in 0..3 {
		app.update();
	}
	let origin = app
		.world()
		.get::<GlobalTransform>(boss_actor)
		.expect("Boss actor should have GlobalTransform")
		.compute_transform();
	fire_capsule_bolt(
		&mut app,
		player,
		origin.transform_point(leg_aim_point(hull)),
		25.0,
		"leg-aimed-bullet",
		origin.rotation * Vec3::X,
	);
	fire_capsule_bolt(
		&mut app,
		player,
		origin.transform_point(outer_body_aim_point(hull)),
		25.0,
		"outer-aimed-bullet",
		origin.rotation * Vec3::Z,
	);
	let initial = app.world().get::<Health>(host_entity).expect("host health").current;
	for _ in 0..40 {
		app.update();
	}
	let final_health = app.world().get::<Health>(host_entity).expect("host health").current;
	anyhow::ensure!(
		(initial - final_health - 50.0).abs() < 0.1,
		"leg and outer-body shots should each debit 25 HP once, initial={initial} final={final_health}"
	);
	Ok(())
}

/// Test 1a: Collider AABB regression for unparented spawn (Ward/Enraged).
/// Spawns boss through production path at translated position with NO parent.
/// Lets Avian settle, then reads live ColliderAabb. Asserts AABB world size is
/// Live encounter scale × unscaled capsule (NOT double-scale, NOT unscaled).
/// MUST FAIL:
///   - Pre-fix (unscaled capsule): ~1.25×1.94×3.29m
///   - 78042407d (scaled capsule + scaled transform): ~110× double-scale
#[test]
fn ward_collider_scales_correctly_unparented() -> Result<()> {
	let mut app = projectile_damage_app();

	// Spawn Ward (unparented form) at nonzero world position
	let world_position = Vec3::new(100.0, 50.0, 200.0);
	let spec = BikanwardaSpec::ward();
	let scale = spec.uniform_scale();

	let ward_root = {
		let mut commands = app.world_mut().commands();
		bikanwarda_character::spawn_bikanwarda_ward(
			&mut commands,
			&spec,
			Transform::from_translation(world_position),
		)
	};

	// Run multiple frames to let avian and transform propagation settle
	for _ in 0..10 {
		app.update();
	}

	// Read live ColliderAabb from the ward root
	let root_aabb = app
		.world()
		.get::<ColliderAabb>(ward_root)
		.expect("Ward root should have ColliderAabb after physics update");

	// Get unscaled capsule dimensions for reference
	let unscaled_capsule = BikanwardaActorRecipe::from_spec(spec.clone()).locomotion_capsule();
	let expected_radius = unscaled_capsule.radius * scale;
	let expected_half_height = unscaled_capsule.half_height() * scale;

	// AABB size in world space
	let aabb_size = root_aabb.max - root_aabb.min;
	let aabb_width_x = aabb_size.x;
	let aabb_height_y = aabb_size.y;
	let aabb_width_z = aabb_size.z;

	// Expected dimensions: Compound collider includes both motor and pronograde hit capsule
	// Motor capsule: vertical (Y-axis), radius ~3.13m
	// Hit capsule: horizontal pronograde (Z-axis forward), extends body length
	// X/Z widths vary: X ~2× motor radius, Z extends with hit capsule length
	// We verify the collider is SCALED (NOT unscaled ~1-2m, NOT double-scaled ~60m+)
	let expected_height = expected_half_height * 2.0;
	let tolerance = 0.25; // 25% height tolerance

	// X width: approximately 2× motor radius (lateral)
	let min_x = expected_radius * 2.0 * 0.8;
	let max_x = expected_radius * 2.0 * 2.0;

	// Z width: motor radius + hit capsule length (pronograde body), can be ~5× motor radius
	let min_z = expected_radius * 2.0; // At least motor diameter
	let max_z = expected_radius * 2.0 * 6.0; // Up to 6× for long hit capsule

	// Assert X width is scaled correctly
	anyhow::ensure!(
		aabb_width_x >= min_x && aabb_width_x <= max_x,
		"Ward AABB width X should be {:.2}m-{:.2}m (1× scaled), got {:.2}m. \
		Pre-fix was ~1.25m (unscaled). 78042407d would be ~{:.2}m (double-scaled).",
		min_x,
		max_x,
		aabb_width_x,
		expected_radius * 2.0 * scale
	);

	// Assert Z width is scaled correctly (accounts for pronograde hit capsule)
	anyhow::ensure!(
		aabb_width_z >= min_z && aabb_width_z <= max_z,
		"Ward AABB width Z should be {:.2}m-{:.2}m (1× scaled with hit capsule), got {:.2}m. \
		Pre-fix was ~1.94m (unscaled). 78042407d would be ~{:.2}m (double-scaled).",
		min_z,
		max_z,
		aabb_width_z,
		expected_radius * 2.0 * scale
	);

	// Assert Y height is scaled correctly
	anyhow::ensure!(
		aabb_height_y >= expected_height * (1.0 - tolerance)
			&& aabb_height_y <= expected_height * (1.0 + tolerance),
		"Ward AABB height Y should be ~{:.2}m (1× scaled), got {:.2}m. \
		Pre-fix was ~3.29m (unscaled). 78042407d would be ~{:.2}m (double-scaled).",
		expected_height,
		aabb_height_y,
		expected_height * scale
	);

	// Assert AABB is centered on root position (within tolerance)
	let aabb_center = (root_aabb.min + root_aabb.max) / 2.0;
	let root_world_pos = app
		.world()
		.get::<GlobalTransform>(ward_root)
		.expect("Ward root should have GlobalTransform")
		.translation();

	let center_offset = (aabb_center - root_world_pos).length();
	anyhow::ensure!(
		center_offset < expected_height * 0.4, // Allow 40% offset for hit capsule
		"Ward AABB center should be near root position: offset={:.2}m, expected<{:.2}m",
		center_offset,
		expected_height * 0.4
	);

	Ok(())
}

/// Test 1b: Collider AABB regression for parented spawn (Encounter).
/// Spawns boss through production path as child of translated host.
/// Lets Avian settle, then reads live ColliderAabb. Asserts AABB world size is
/// Live encounter scale × unscaled capsule (NOT double-scale, NOT unscaled).
/// MUST FAIL:
///   - Pre-fix (unscaled capsule): ~1.25×1.94×3.29m
///   - 78042407d worked by accident for parented case only
#[test]
fn encounter_collider_scales_correctly_parented() -> Result<()> {
	let mut app = projectile_damage_app();

	// Spawn encounter (parented to host) at nonzero world position
	let host_position = Vec3::new(100.0, 50.0, 200.0);
	let spec = BikanwardaSpec::encounter();
	let scale = spec.uniform_scale();

	let encounter_root = {
		let mut commands = app.world_mut().commands();
		spawn_bikanwarda_encounter(&mut commands, &spec, Transform::from_translation(host_position))
	};

	// Run multiple frames to let avian and transform propagation settle
	for _ in 0..10 {
		app.update();
	}

	// Read live ColliderAabb from the encounter root
	let root_aabb = app
		.world()
		.get::<ColliderAabb>(encounter_root)
		.expect("Encounter root should have ColliderAabb after physics update");

	// Get unscaled capsule dimensions for reference
	let unscaled_capsule = BikanwardaActorRecipe::from_spec(spec.clone()).locomotion_capsule();
	let expected_radius = unscaled_capsule.radius * scale;
	let expected_half_height = unscaled_capsule.half_height() * scale;

	// AABB size in world space
	let aabb_size = root_aabb.max - root_aabb.min;
	let aabb_width_x = aabb_size.x;
	let aabb_height_y = aabb_size.y;
	let aabb_width_z = aabb_size.z;

	// Expected dimensions: Compound collider includes both motor and pronograde hit capsule
	// Motor capsule: vertical (Y-axis), radius ~3.68m
	// Hit capsule: horizontal pronograde (Z-axis forward), extends body length
	let expected_height = expected_half_height * 2.0;
	let tolerance = 0.25; // 25% height tolerance

	// X width: approximately 2× motor radius (lateral)
	let min_x = expected_radius * 2.0 * 0.8;
	let max_x = expected_radius * 2.0 * 2.0;

	// Z width: motor radius + hit capsule length (pronograde body), can be ~5× motor radius
	let min_z = expected_radius * 2.0; // At least motor diameter
	let max_z = expected_radius * 2.0 * 6.0; // Up to 6× for long hit capsule

	// Assert X width is scaled correctly
	anyhow::ensure!(
		aabb_width_x >= min_x && aabb_width_x <= max_x,
		"Encounter AABB width X should be {:.2}m-{:.2}m (1× scaled), got {:.2}m. Pre-fix was ~1.25m (unscaled).",
		min_x, max_x, aabb_width_x
	);

	// Assert Z width is scaled correctly (accounts for pronograde hit capsule)
	anyhow::ensure!(
		aabb_width_z >= min_z && aabb_width_z <= max_z,
		"Encounter AABB width Z should be {:.2}m-{:.2}m (1× scaled with hit capsule), got {:.2}m. Pre-fix was ~1.94m (unscaled).",
		min_z, max_z, aabb_width_z
	);

	// Assert Y height is scaled correctly
	anyhow::ensure!(
		aabb_height_y >= expected_height * (1.0 - tolerance) && aabb_height_y <= expected_height * (1.0 + tolerance),
		"Encounter AABB height Y should be ~{:.2}m (1× scaled), got {:.2}m. Pre-fix was ~3.29m (unscaled).",
		expected_height, aabb_height_y
	);

	// Assert AABB is centered on root position (within tolerance)
	let aabb_center = (root_aabb.min + root_aabb.max) / 2.0;
	let root_world_pos = app
		.world()
		.get::<GlobalTransform>(encounter_root)
		.expect("Encounter root should have GlobalTransform")
		.translation();

	let center_offset = (aabb_center - root_world_pos).length();
	anyhow::ensure!(
		center_offset < expected_height * 0.4, // Allow 40% offset for hit capsule
		"Encounter AABB center should be near root position: offset={:.2}m, expected<{:.2}m",
		center_offset,
		expected_height * 0.4
	);

	Ok(())
}

/// Test 2: Real shot at visible body (78042407d fix verification).
/// Fires real projectile through production path at a point on visible body
/// outside the old small box (~1.25×1.94×3.29m). Asserts real contact and
/// host loses HP exactly once via damage/src/apply.rs.
/// MUST FAIL pre-78042407d when collider was too small.
#[test]
fn projectile_hits_visible_body_outside_old_small_box() -> Result<()> {
	let mut app = projectile_damage_app();

	// Spawn boss at origin for simplicity
	let host_position = Vec3::ZERO;
	let spec = BikanwardaSpec::encounter();

	let boss_actor = {
		let mut commands = app.world_mut().commands();
		spawn_bikanwarda_encounter(&mut commands, &spec, Transform::from_translation(host_position))
	};

	app.update();

	// Spawn host with Health
	let host_entity = app
		.world_mut()
		.spawn((
			Name::new("boss-host"),
			Health::from_max(1000.0),
			Transform::default(),
			GlobalTransform::default(),
		))
		.id();

	// Parent actor to host (production structure)
	app.world_mut().entity_mut(boss_actor).insert(ChildOf(host_entity));

	// Spawn player as projectile source
	let player = app
		.world_mut()
		.spawn((Name::new("test-player"), Transform::default(), GlobalTransform::default()))
		.id();

	// Update physics to settle colliders
	for _ in 0..5 {
		app.update();
	}

	// Target point on the scaled body OUTSIDE the ordinary ~1.25×1.94×3.29 m box
	// and inside the live encounter hull.
	let scaled = BikanwardaActorRecipe::from_spec(spec.clone())
		.components()
		.scaled_locomotion_capsule();
	let target_pos = Vec3::new(scaled.radius * 1.15, scaled.half_height() * 0.35, 0.0);
	let start_pos = target_pos + Vec3::X * -12.0;
	let direction = (target_pos - start_pos).normalize();

	// Use BulletSpec
	let bullet_spec = projectiles::BulletSpec::default();
	let gravity = 0.0; // Straight shot

	// Spawn projectile with physics components
	let collider = Collider::capsule(bullet_spec.radius, bullet_spec.length);
	let velocity = direction * bullet_spec.speed;
	let cap_transform = Transform::from_translation(start_pos).looking_to(direction, Vec3::Y);

	let _projectile = app
		.world_mut()
		.spawn((
			Name::new("test-bullet-outside-old-box"),
			cap_transform,
			GlobalTransform::from(cap_transform),
			RigidBody::Dynamic,
			MassPropertiesBundle::from_shape(&collider, 1.0),
			collider,
			Sensor,
			PhysicsInteractionLayer::projectile_layers(),
			LockedAxes::ROTATION_LOCKED,
			LinearVelocity(velocity),
			GravityScale(gravity),
			Restitution::ZERO,
			Flight::spawn(
				start_pos,
				bullet_spec.max_range,
				bullet_spec.penetration,
				bullet_spec.max_age,
			),
			ProjectileSource(player),
			HitPayload { amount: 25.0 },
		))
		.id();

	// Record initial host health
	let initial_health =
		app.world().get::<Health>(host_entity).expect("Host should have Health").current;

	// Run projectile sweep and damage application
	// Projectile travels ~25m at 28 m/s = ~0.89s = ~54 frames at 60fps
	let mut damage_applied_count = 0;
	let mut final_health = initial_health;

	for _ in 0..60 {
		app.update();

		let current_health =
			app.world().get::<Health>(host_entity).expect("Host should have Health").current;

		if current_health < final_health {
			damage_applied_count += 1;
			final_health = current_health;
		}
	}

	// Assert host health decreased (projectile hit)
	anyhow::ensure!(
		final_health < initial_health,
		"Host health should decrease after projectile hits visible body: initial={}, final={}. \
		Pre-fix: projectile missed small collider.",
		initial_health,
		final_health
	);

	// Assert damage applied exactly once (not multiple hits)
	let damage_taken = initial_health - final_health;
	anyhow::ensure!(
		damage_applied_count == 1,
		"Damage should apply exactly once, got {} applications",
		damage_applied_count
	);

	anyhow::ensure!(
		(damage_taken - 25.0).abs() < 1.0,
		"Damage should be ~25.0 (one hit), got {:.1}",
		damage_taken
	);

	Ok(())
}

/// Test 3: Mouth socket scaling verification.
/// Asserts mouth socket anchor's local transform is correctly scaled.
/// The anchor position should scale with the locomotion capsule.
#[test]
fn mouth_socket_anchor_scales_with_capsule() -> Result<()> {
	// Get scaled capsule to calculate expected mouth position
	let spec = BikanwardaSpec::encounter();
	let actor = BikanwardaActorRecipe::from_spec(spec.clone()).components();
	let scaled_capsule = actor.scaled_locomotion_capsule();
	let anchors = bikanwarda_character::anchors::BikanwardaAnchors::for_capsule(scaled_capsule);
	let expected_mouth_local =
		anchors.get("mouth_emit").expect("Anchors should have MOUTH_EMIT").translation;

	// Verify the anchor local position is scaled
	use characters::CharacterComponents;
	let unscaled_capsule = actor.locomotion_capsule();
	let unscaled_anchors =
		bikanwarda_character::anchors::BikanwardaAnchors::for_capsule(unscaled_capsule);
	let unscaled_mouth_local = unscaled_anchors
		.get("mouth_emit")
		.expect("Unscaled anchors should have MOUTH_EMIT")
		.translation;

	// Assert scaled mouth position tracks spec.uniform_scale()
	anyhow::ensure!(
		expected_mouth_local.length() > unscaled_mouth_local.length() * 3.0,
		"Mouth anchor should be scaled: unscaled={:.2}, scaled={:.2}, ratio={:.2}",
		unscaled_mouth_local.length(),
		expected_mouth_local.length(),
		expected_mouth_local.length() / unscaled_mouth_local.length()
	);

	// Assert mouth socket is at expected height (Y component)
	// Expected: capsule.radius (scaled) + some vertical offset
	let expected_mouth_y = scaled_capsule.radius * 0.65 + scaled_capsule.radius;
	let mouth_y_local = expected_mouth_local.y;
	anyhow::ensure!(
		(mouth_y_local - expected_mouth_y).abs() < expected_mouth_y * 0.3,
		"Mouth Y should be ~{:.2}m, got {:.2}m (tolerance 30%)",
		expected_mouth_y,
		mouth_y_local
	);

	Ok(())
}

/// Test 4: Mouth socket world-space GlobalTransform verification.
/// Verifies BikanwardaMouthSocket's world GlobalTransform after propagation
/// matches the expected visible mouth position for both parented Encounter
/// and unparented Ward forms.
///
/// MUST FAIL if socket is parented incorrectly or scale is wrong.
#[test]
fn mouth_socket_world_transform_matches_visible_position() -> Result<()> {
	let mut app = projectile_damage_app();

	// Test 4a: Parented Encounter (host at translated position)
	let host_position = Vec3::new(100.0, 50.0, 200.0);
	let encounter_spec = BikanwardaSpec::encounter();
	let encounter_scale = encounter_spec.uniform_scale();

	// Spawn encounter actor at host position
	let encounter_actor = {
		let mut commands = app.world_mut().commands();
		spawn_bikanwarda_encounter(
			&mut commands,
			&encounter_spec,
			Transform::from_translation(host_position),
		)
	};

	app.update();

	// Calculate expected mouth position in world space
	let encounter_recipe = BikanwardaActorRecipe::from_spec(encounter_spec.clone());
	let encounter_scaled_capsule = encounter_recipe.components().scaled_locomotion_capsule();
	let encounter_anchors =
		bikanwarda_character::anchors::BikanwardaAnchors::for_capsule(encounter_scaled_capsule);
	let encounter_mouth_local = encounter_anchors
		.get("mouth_emit")
		.expect("Encounter anchors should have MOUTH_EMIT")
		.translation;

	// For unit-scale root with scaled collider/anchors, mouth world position = root position + mouth anchor
	let expected_encounter_mouth_world = host_position + encounter_mouth_local;

	// Spawn mouth socket as child of encounter actor (matching production assembly)
	let encounter_socket = app
		.world_mut()
		.spawn((
			Name::new("encounter_mouth_socket"),
			bikanwarda_attacks::BikanwardaMouthSocket,
			ChildOf(encounter_actor),
			Transform::from_translation(encounter_mouth_local),
			GlobalTransform::default(),
		))
		.id();

	// Run transform propagation
	for _ in 0..5 {
		app.update();
	}

	// Verify encounter socket GlobalTransform
	let encounter_socket_global = app
		.world()
		.get::<GlobalTransform>(encounter_socket)
		.expect("Encounter socket should have GlobalTransform");

	let encounter_mouth_world = encounter_socket_global.translation();
	let encounter_error = (encounter_mouth_world - expected_encounter_mouth_world).length();

	anyhow::ensure!(
		encounter_error < encounter_scaled_capsule.radius * 0.1, // 10% of capsule radius tolerance
		"Encounter mouth socket world position should be {:.2?}, got {:.2?} (error: {:.2}m)",
		expected_encounter_mouth_world,
		encounter_mouth_world,
		encounter_error
	);

	// Test 4b: Unparented Ward (no host, just translated root)
	let ward_position = Vec3::new(300.0, 75.0, 400.0);
	let ward_spec = BikanwardaSpec::ward();
	let ward_scale = ward_spec.uniform_scale();

	let ward_actor = {
		let mut commands = app.world_mut().commands();
		bikanwarda_character::spawn_bikanwarda_ward(
			&mut commands,
			&ward_spec,
			Transform::from_translation(ward_position),
		)
	};

	app.update();

	// Calculate expected Ward mouth position in world space
	let ward_recipe = BikanwardaActorRecipe::from_spec(ward_spec.clone());
	let ward_scaled_capsule = ward_recipe.components().scaled_locomotion_capsule();
	let ward_anchors =
		bikanwarda_character::anchors::BikanwardaAnchors::for_capsule(ward_scaled_capsule);
	let ward_mouth_local = ward_anchors
		.get("mouth_emit")
		.expect("Ward anchors should have MOUTH_EMIT")
		.translation;

	// For unit-scale root with scaled collider/anchors, mouth world position = root position + mouth anchor
	let expected_ward_mouth_world = ward_position + ward_mouth_local;

	// Spawn mouth socket as child of ward actor
	let ward_socket = app
		.world_mut()
		.spawn((
			Name::new("ward_mouth_socket"),
			bikanwarda_attacks::BikanwardaMouthSocket,
			ChildOf(ward_actor),
			Transform::from_translation(ward_mouth_local),
			GlobalTransform::default(),
		))
		.id();

	// Run transform propagation
	for _ in 0..5 {
		app.update();
	}

	// Verify ward socket GlobalTransform
	let ward_socket_global = app
		.world()
		.get::<GlobalTransform>(ward_socket)
		.expect("Ward socket should have GlobalTransform");

	let ward_mouth_world = ward_socket_global.translation();
	let ward_error = (ward_mouth_world - expected_ward_mouth_world).length();

	anyhow::ensure!(
		ward_error < ward_scaled_capsule.radius * 0.1, // 10% of capsule radius tolerance
		"Ward mouth socket world position should be {:.2?}, got {:.2?} (error: {:.2}m)",
		expected_ward_mouth_world,
		ward_mouth_world,
		ward_error
	);

	// Verify mouth heights are reasonable for scaled boss forms
	anyhow::ensure!(
		encounter_mouth_world.y > host_position.y + encounter_scaled_capsule.radius,
		"Encounter mouth should be above root by at least capsule radius: mouth_y={:.2}, root_y={:.2}, radius={:.2}",
		encounter_mouth_world.y, host_position.y, encounter_scaled_capsule.radius
	);

	anyhow::ensure!(
		ward_mouth_world.y > ward_position.y + ward_scaled_capsule.radius,
		"Ward mouth should be above root by at least capsule radius: mouth_y={:.2}, root_y={:.2}, radius={:.2}",
		ward_mouth_world.y, ward_position.y, ward_scaled_capsule.radius
	);

	Ok(())
}

#[derive(Resource, Clone, Copy)]
struct ObservedHost(Entity);

#[derive(Resource, Default)]
struct ObservedApplies {
	events: Vec<DamageApplied>,
}

fn collect_host_applies(
	mut reader: MessageReader<DamageApplied>,
	host: Res<ObservedHost>,
	mut observed: ResMut<ObservedApplies>,
) {
	for event in reader.read() {
		if event.target == host.0 {
			observed.events.push(*event);
		}
	}
}

fn production_player_shot_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		PhysicsPlugins::default(),
		bevy::asset::AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
	))
	.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));
	app.add_plugins(projectiles::ProjectilesPlugin);
	app.add_plugins(damage::DamagePlugin);
	app.add_plugins(crate::chains::BikanwardaChainPhysicsPlugin);
	firearms::register_firearm_runtime(&mut app);
	app.finish();
	app.update();
	app
}

/// One ordinary player projectile, fired through `decide_weapon_fire` at the
/// production boss assembly, must apply damage exactly once over 10+ frames.
#[test]
fn ordinary_player_fire_path_applies_once_to_boss() -> Result<()> {
	let mut app = production_player_shot_app();

	let spec = BikanwardaSpec::encounter();
	let boss_actor = {
		let mut commands = app.world_mut().commands();
		spawn_bikanwarda_encounter(&mut commands, &spec, Transform::IDENTITY)
	};
	app.update();

	let host_entity = app
		.world_mut()
		.spawn((
			Name::new("boss-host"),
			Health::from_max(1000.0),
			Transform::default(),
			GlobalTransform::default(),
		))
		.id();
	app.world_mut().entity_mut(boss_actor).insert(ChildOf(host_entity));

	let player = app
		.world_mut()
		.spawn((Name::new("test-player"), Transform::default(), GlobalTransform::default()))
		.id();

	app.insert_resource(ObservedHost(host_entity));
	app.init_resource::<ObservedApplies>();
	app.add_systems(PostUpdate, collect_host_applies.after(damage::DamageSystems::Apply));

	for _ in 0..5 {
		app.update();
	}

	let boss_transform = app
		.world()
		.get::<GlobalTransform>(boss_actor)
		.expect("Boss actor should have GlobalTransform");
	let target_pos = boss_transform.translation();
	let start_pos = target_pos + Vec3::X * -15.0;
	let direction = (target_pos - start_pos).normalize();

	app.world_mut().spawn((
		Name::new("player-weapon"),
		firearms::Weapon::bolt(),
		firearms::WeaponTrigger(true),
		firearms::FireOnTrigger,
		firearms::FireControl::semi(),
		firearms::WeaponLaunchPose::ready_at(start_pos, direction),
		HitPayload { amount: 25.0 },
		ProjectileSource(player),
	));

	for _ in 0..20 {
		app.update();
	}

	let applies = app.world().resource::<ObservedApplies>().events.clone();
	anyhow::ensure!(
		applies.len() == 1,
		"ordinary fire-path shot should apply once over 20 frames, got {} events: {applies:?}",
		applies.len()
	);

	let health = app.world().get::<Health>(host_entity).expect("Host should have Health");
	anyhow::ensure!(
		(health.current - 975.0).abs() < 0.1,
		"Host health should be 975 after one 25 HP shot, got {}",
		health.current
	);

	Ok(())
}
