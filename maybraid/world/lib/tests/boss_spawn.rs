//! Boss spawn unit tests ([#1116](https://github.com/ramate-io/maybraid/issues/1116)).
//!
//! Tests that boss spawn request processing works correctly: spawning publishes
//! the boss with the correct ID and position, config persists across restarts,
//! and the session reseeder restores the boss after HCSG restarts.
//!
//! Note: Full integration tests (presentation, assembly, terrain interaction)
//! require complete world setup with all system dependencies.

use bevy::prelude::*;
use bikanwarda_encounter::{BikanwardaBossEncounter, chained_encounter_id};
use lod::gen::Bytes;
use lod::hcsg::{HcsgRestartRequest, HcsgStorage};
use maybraid_world::boss_spawn::{BossSpawnConfig, BossSpawnPlugin, RequestSpawnBoss};

/// Set up a minimal app with boss spawn plugin
fn minimal_app() -> App {
	let mut app = App::new();
	app.add_plugins(bevy::MinimalPlugins)
		.add_plugins(BossSpawnPlugin);
	app.update();
	app
}

/// Check if a boss exists in storage with the correct identity and position
fn boss_in_storage(storage: &HcsgStorage, expected_position: Vec3) -> anyhow::Result<bool> {
	let id = lod::gen::Id::Bytes(Bytes(chained_encounter_id().as_bytes()));
	let entry = storage.try_entry::<BikanwardaBossEncounter>(id)
		.map_err(|_| anyhow::anyhow!("storage busy"))?;
	
	match entry {
		Some(e) => {
			let pos_match = (e.value.boss_position - expected_position).length() < 0.1;
			Ok(pos_match)
		}
		None => Ok(false),
	}
}

/// Test that boss spawn config resource is initialized
#[test]
fn boss_spawn_config_initialized() {
	let app = minimal_app();
	let config = app.world().resource::<BossSpawnConfig>();
	assert!(!config.active, "Config should start inactive");
	assert_eq!(config.position, Vec3::ZERO, "Config should start at origin");
}

/// Test that boss spawn request is processed and removed
#[test]
fn boss_spawn_request_is_consumed() {
	let mut app = minimal_app();
	
	let spawn_pos = Vec2::new(1746.8, 1025.9);
	app.world_mut().spawn(RequestSpawnBoss { at: Some(spawn_pos) });
	
	// Count requests before processing
	let before = app.world_mut().query::<&RequestSpawnBoss>().iter(app.world()).count();
	assert_eq!(before, 1, "Should have one spawn request");
	
	app.update();
	
	// After processing, request should be despawned
	let after = app.world_mut().query::<&RequestSpawnBoss>().iter(app.world()).count();
	assert_eq!(after, 0, "Spawn request should be consumed");
}

/// Test that boss spawn publishes to storage with correct ID immediately
#[test]
fn boss_spawn_publishes_with_correct_id() -> anyhow::Result<()> {
	let mut app = minimal_app();
	
	let spawn_pos = Vec3::new(1746.8, 0.0, 1025.9);
	app.world_mut().spawn(RequestSpawnBoss {
		at: Some(Vec2::new(spawn_pos.x, spawn_pos.z)),
	});
	
	app.update();
	
	// Check storage immediately
	let storage = app.world().resource::<HcsgStorage>();
	anyhow::ensure!(
		boss_in_storage(storage, spawn_pos)?,
		"Boss should be published to storage with correct ID and position"
	);
	
	Ok(())
}

/// Test that boss spawn updates config resource
#[test]
fn boss_spawn_updates_config() -> anyhow::Result<()> {
	let mut app = minimal_app();
	
	let spawn_pos = Vec3::new(1746.8, 0.0, 1025.9);
	app.world_mut().spawn(RequestSpawnBoss {
		at: Some(Vec2::new(spawn_pos.x, spawn_pos.z)),
	});
	
	app.update();
	
	// Verify config was updated
	let config = app.world().resource::<BossSpawnConfig>();
	anyhow::ensure!(config.active, "Config should be active after spawn");
	anyhow::ensure!(
		(config.position - spawn_pos).length() < 0.1,
		"Config position should match spawn position"
	);
	
	Ok(())
}

/// Test that session reseeder restores boss after HCSG restart
#[test]
fn boss_survives_session_restart() -> anyhow::Result<()> {
	let mut app = minimal_app();
	
	let spawn_pos = Vec3::new(1746.8, 0.0, 1025.9);
	
	// Spawn boss
	app.world_mut().spawn(RequestSpawnBoss {
		at: Some(Vec2::new(spawn_pos.x, spawn_pos.z)),
	});
	app.update();
	
	// Verify boss is in storage before restart
	let storage = app.world().resource::<HcsgStorage>();
	anyhow::ensure!(
		boss_in_storage(storage, spawn_pos)?,
		"Boss should be in storage before restart"
	);
	
	// Request HCSG session restart
	app.world_mut().resource_mut::<HcsgRestartRequest>().request();
	app.update();
	
	// Verify boss is reseeded after restart
	let storage = app.world().resource::<HcsgStorage>();
	anyhow::ensure!(
		boss_in_storage(storage, spawn_pos)?,
		"Boss should be reseeded after restart"
	);
	
	Ok(())
}

/// Test that second spawn replaces first
#[test]
fn second_spawn_replaces_first() -> anyhow::Result<()> {
	let mut app = minimal_app();
	
	let first_pos = Vec3::new(100.0, 0.0, 200.0);
	app.world_mut().spawn(RequestSpawnBoss {
		at: Some(Vec2::new(first_pos.x, first_pos.z)),
	});
	app.update();
	
	let storage = app.world().resource::<HcsgStorage>();
	anyhow::ensure!(
		boss_in_storage(storage, first_pos)?,
		"First boss should be at first position"
	);
	
	// Spawn at second position
	let second_pos = Vec3::new(300.0, 0.0, 400.0);
	app.world_mut().spawn(RequestSpawnBoss {
		at: Some(Vec2::new(second_pos.x, second_pos.z)),
	});
	app.update();
	
	// Boss should now be at second position
	let storage = app.world().resource::<HcsgStorage>();
	anyhow::ensure!(
		boss_in_storage(storage, second_pos)?,
		"Boss should be at second position"
	);
	
	// Config should reflect second position
	let config = app.world().resource::<BossSpawnConfig>();
	anyhow::ensure!(
		(config.position - second_pos).length() < 0.1,
		"Config should be updated to second position"
	);
	
	Ok(())
}

/// Test that boss has correct encounter ID
#[test]
fn boss_uses_chained_encounter_id() -> anyhow::Result<()> {
	let mut app = minimal_app();
	
	let spawn_pos = Vec3::new(1746.8, 0.0, 1025.9);
	app.world_mut().spawn(RequestSpawnBoss {
		at: Some(Vec2::new(spawn_pos.x, spawn_pos.z)),
	});
	app.update();
	
	let storage = app.world().resource::<HcsgStorage>();
	let id = lod::gen::Id::Bytes(Bytes(chained_encounter_id().as_bytes()));
	let entry = storage.try_entry::<BikanwardaBossEncounter>(id)
		.map_err(|_| anyhow::anyhow!("storage busy"))?;
	
	anyhow::ensure!(entry.is_some(), "Boss should use chained encounter ID");
	anyhow::ensure!(
		entry.unwrap().value.id == chained_encounter_id(),
		"Boss ID should match chained_encounter_id()"
	);
	
	Ok(())
}
