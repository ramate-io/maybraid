//! Player projectile damage integration tests.

use anyhow::Result;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::time::{TimeUpdateStrategy, Virtual};
use bikanwarda_character::{spawn_bikanwarda_encounter, BikanwardaRoot, BikanwardaSpec};
use damage::{Health, HitPayload};
use lod_avian::PhysicsInteractionLayer;
use projectiles::{BulletSpec, Flight, ProjectileSource};
use std::time::Duration;
use bevy::math::Dir3;

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
		.spawn((
			Name::new("test-player"),
			Transform::default(),
			GlobalTransform::default(),
		))
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
	
	// Fire a projectile at the boss (aiming at center mass)
	let target_pos = boss_transform.translation() + Vec3::Y * 10.0; // Aim at boss center
	let start_pos = target_pos + Vec3::X * -15.0; // 15m away horizontally
	let direction = (target_pos - start_pos).normalize();
	
	// Use BulletSpec
	let spec = BulletSpec::default();
	let gravity = 0.0; // Straight shot for test simplicity
	
	// Spawn projectile with physics components (required for movement)
	let collider = Collider::capsule(spec.radius, spec.length);
	let velocity = direction * spec.speed;
	// capsule_along_y takes Vec3 direction (not Dir3)
	let muzzle_to_target = direction;
	let cap_transform = Transform::from_translation(start_pos).looking_to(muzzle_to_target, Vec3::Y);
	
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
	let initial_health = app
		.world()
		.get::<Health>(host_entity)
		.expect("Host should have Health")
		.current;
	
	// Run projectile sweep and damage application for several frames
	// Boss extends from X=-6.39 to X=+6.39, projectile starts at X=-15
	// At 28 m/s, projectile needs ~8.6m / 28m/s = 0.31s = ~18 frames at 60fps
	for _ in 0..40 {
		app.update();
	}
	
	// Check host health after projectile passage
	let final_health = app
		.world()
		.get::<Health>(host_entity)
		.expect("Host should have Health")
		.current;
	
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

