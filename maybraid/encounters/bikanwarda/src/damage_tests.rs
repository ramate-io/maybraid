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
	
	// Spawn boss at origin with production collider and Health
	let spec = BikanwardaSpec::encounter();
	let mut commands = app.world_mut().commands();
	let boss_entity = spawn_bikanwarda_encounter(&mut commands, &spec, Transform::IDENTITY);
	
	// Add Health to boss (matches production setup where host has Health)
	app.world_mut()
		.entity_mut(boss_entity)
		.insert(Health::from_max(1000.0));
	
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
	
	// Get boss GlobalTransform to aim at
	let boss_transform = app
		.world()
		.get::<GlobalTransform>(boss_entity)
		.expect("Boss should have GlobalTransform");
	
	// Fire a projectile at the boss (aiming at center mass)
	let target_pos = boss_transform.translation() + Vec3::Y * 10.0; // Aim at boss center
	let start_pos = target_pos + Vec3::X * -15.0; // 15m away horizontally
	let direction = (target_pos - start_pos).normalize();
	
	// Use BulletSpec (has gravity, more realistic)
	let spec = BulletSpec::default();
	
	// Spawn projectile (projectiles use Transform-based movement, not a separate Velocity component)
	let projectile = app
		.world_mut()
		.spawn((
			Name::new("test-bullet"),
			Transform::from_translation(start_pos),
			GlobalTransform::from_translation(start_pos),
			Collider::capsule(spec.length / 2.0, spec.radius),
			Flight::spawn(start_pos, spec.max_range, spec.penetration, spec.max_age),
			ProjectileSource(player),
			// Add damage payload so ProjectileContact → Hit conversion works
			HitPayload { amount: 50.0 },
		))
		.id();
	
	// Record initial boss health
	let initial_health = app
		.world()
		.get::<Health>(boss_entity)
		.expect("Boss should have Health")
		.current;
	
	// Run projectile sweep and damage application for several frames
	// Projectile should hit boss within ~20 frames (at 180m/s, 15m distance = ~0.08s = ~5 frames)
	for _ in 0..20 {
		app.update();
	}
	
	// Check boss health after projectile passage
	let final_health = app
		.world()
		.get::<Health>(boss_entity)
		.expect("Boss should have Health")
		.current;
	
	anyhow::ensure!(
		final_health < initial_health,
		"Boss health should decrease after projectile hit: initial={}, final={}",
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

/// Test that projectile sweep produces ProjectileContact when hitting Fixed colliders.
#[test]
fn projectile_sweeps_hit_fixed_colliders() -> Result<()> {
	let mut app = projectile_damage_app();
	
	// Spawn fixed wall
	let wall = app
		.world_mut()
		.spawn((
			Name::new("wall"),
			Transform::from_translation(Vec3::X * 10.0),
			GlobalTransform::from_translation(Vec3::X * 10.0),
			RigidBody::Static,
			Collider::cuboid(2.0, 5.0, 5.0),
			PhysicsInteractionLayer::fixed_layers(),
		))
		.id();
	
	// Spawn player
	let player = app
		.world_mut()
		.spawn((Name::new("shooter"), Transform::default(), GlobalTransform::default()))
		.id();
	
	// Update to settle physics
	for _ in 0..3 {
		app.update();
	}
	
	// Spawn projectile aimed at wall
	let spec = projectiles::BoltSpec::default();
	let start_pos = Vec3::ZERO;
	let direction = Dir3::X;
	
	let _projectile = app
		.world_mut()
		.spawn((
			Name::new("bolt"),
			Transform::from_translation(start_pos),
			GlobalTransform::from_translation(start_pos),
			Collider::capsule(spec.length / 2.0, spec.radius),
			Flight::spawn(start_pos, spec.max_range, spec.penetration, spec.max_age),
			ProjectileSource(player),
		))
		.id();
	
	// Run projectile sweep
	for _ in 0..10 {
		app.update();
	}
	
	// Check for ProjectileContact messages
	let contacts: Vec<_> = app
		.world_mut()
		.resource_mut::<Messages<projectiles::ProjectileContact>>()
		.drain()
		.collect();
	
	anyhow::ensure!(!contacts.is_empty(), "Should have at least one ProjectileContact");
	
	// Verify contact hit the wall
	let hit_wall = contacts.iter().any(|contact| contact.target == wall);
	anyhow::ensure!(hit_wall, "Projectile should hit the wall");
	
	Ok(())
}
