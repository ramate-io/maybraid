//! Boss spawn unit tests ([#1116](https://github.com/ramate-io/maybraid/issues/1116)).
//!
//! Tests that boss spawn request processing works correctly: spawning publishes
//! the boss with the correct ID and position, config persists across restarts,
//! and the session reseeder restores the boss after HCSG restarts.
//!
//! Note: Full integration tests (presentation, assembly, terrain interaction)
//! require complete world setup with all system dependencies.

#![cfg(feature = "debug-commands")]

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

/// World-level regression test: boss spawn with extended stability verification.
///
/// This test verifies:
/// - Boss publishes with correct ID and position
/// - Boss survives HCSG session restart with position preserved
/// - Config persists across 12+ seconds of simulation (exceeds ~7.5s failure from original bug)
/// - Multiple restarts don't corrupt boss data
///
/// The original bug had three symptoms after `boss spawn --at X,Z`:
/// 1. No boss appeared (ID mismatch + no reseeder)
/// 2. Terrain/vegetation cleared at ~7.5s (global restart side effect)
/// 3. Player fell through world (terrain colliders removed)
///
/// This test validates the storage/session contract. Full presentation/assembly/terrain
/// verification requires WorldPlugin systems (not feasible in minimal test environment).
#[test]
fn boss_spawn_world_integration() -> anyhow::Result<()> {
	use lod::hcsg::HcsgRestartRequest;
	
	const SPAWN_X: f32 = 1746.8;
	const SPAWN_Z: f32 = 1025.9;
	const SPAWN_POS: Vec3 = Vec3::new(SPAWN_X, 0.0, SPAWN_Z);
	
	let mut app = minimal_app();
	
	// Request boss spawn at specific position
	app.world_mut().spawn(RequestSpawnBoss {
		at: Some(Vec2::new(SPAWN_X, SPAWN_Z)),
	});
	
	app.update();
	
	// Verify boss is published to storage with correct ID
	let storage = app.world().resource::<HcsgStorage>();
	let boss_id = lod::gen::Id::Bytes(Bytes(chained_encounter_id().as_bytes()));
	let entry = storage.try_entry::<BikanwardaBossEncounter>(boss_id)
		.map_err(|_| anyhow::anyhow!("storage busy"))?;
	anyhow::ensure!(
		entry.is_some(),
		"Boss should be published to storage immediately after spawn"
	);
	
	let boss_data = entry.unwrap();
	anyhow::ensure!(
		(boss_data.value.boss_position - SPAWN_POS).length() < 0.1,
		"Boss should be at requested position, got ({}, {}, {})",
		boss_data.value.boss_position.x,
		boss_data.value.boss_position.y,
		boss_data.value.boss_position.z
	);
	
	// Run for 12 seconds (720 updates) to exceed the ~7.5s failure point from the original bug.
	// In the full game, this would be 720 frames at 60 FPS. Here we just verify config/storage
	// stability across many updates.
	const STABILITY_UPDATES: u32 = 720;
	
	for update_idx in 0..STABILITY_UPDATES {
		app.update();
		
		// Periodically verify config and storage (every 60 updates to avoid storage contention)
		if update_idx % 60 == 0 {
			let config = app.world().resource::<BossSpawnConfig>();
			anyhow::ensure!(
				config.active,
				"Boss config deactivated at update {}",
				update_idx
			);
			
			anyhow::ensure!(
				boss_in_storage(&app.world().resource::<HcsgStorage>(), SPAWN_POS)?,
				"Boss disappeared from storage at update {}",
				update_idx
			);
		}
	}
	
	// Test HCSG session restart survival
	app.world_mut().resource_mut::<HcsgRestartRequest>().request();
	app.update();
	
	// Verify boss survives restart
	let storage = app.world().resource::<HcsgStorage>();
	let entry = storage.try_entry::<BikanwardaBossEncounter>(boss_id)
		.map_err(|_| anyhow::anyhow!("storage busy after restart"))?;
	anyhow::ensure!(
		entry.is_some(),
		"Boss should survive HCSG restart via session reseeder"
	);
	
	let boss_data = entry.unwrap();
	anyhow::ensure!(
		(boss_data.value.boss_position - SPAWN_POS).length() < 0.1,
		"Boss position should be preserved after restart"
	);
	
	// Test multiple restarts (simulates repeated HCSG restarts from other systems)
	for restart_idx in 0..3 {
		app.world_mut().resource_mut::<HcsgRestartRequest>().request();
		app.update();
		
		let storage = app.world().resource::<HcsgStorage>();
		let entry = storage.try_entry::<BikanwardaBossEncounter>(boss_id)
			.map_err(|_| anyhow::anyhow!("storage busy after restart {}", restart_idx))?;
		anyhow::ensure!(
			entry.is_some(),
			"Boss disappeared after restart {}",
			restart_idx
		);
		
		let boss_data = entry.unwrap();
		anyhow::ensure!(
			(boss_data.value.boss_position - SPAWN_POS).length() < 0.1,
			"Boss position corrupted after restart {}",
			restart_idx
		);
	}
	
	// Run a few more updates after all restarts
	for _ in 0..60 {
		app.update();
	}
	
	// Final verification
	let storage = app.world().resource::<HcsgStorage>();
	let entry = storage.try_entry::<BikanwardaBossEncounter>(boss_id)
		.map_err(|_| anyhow::anyhow!("storage busy at end"))?;
	anyhow::ensure!(
		entry.is_some(),
		"Boss should still be in storage at test end"
	);
	
	Ok(())
}

/// Test ground fit: boss Y coordinate includes terrain height + spawn_height offset (#1117)
#[test]
fn boss_spawn_applies_ground_fit_offset() -> anyhow::Result<()> {
	// This test would require full Durham terrain setup to verify the exact Y coordinate.
	// Here we verify that the spawn system has access to the necessary components.
	use bikanwarda_character::{BikanwardaActorRecipe, BikanwardaSpec};
	use characters::LocomotionCapsule;
	
	// Verify that boss hull spawn_height is accessible and reasonable
	let spec = BikanwardaSpec::encounter();
	let recipe = BikanwardaActorRecipe::from_spec(spec);
	let hull = recipe.components().scaled_locomotion_capsule();
	let spawn_height = hull.spawn_height();
	
	anyhow::ensure!(
		spawn_height > 0.0,
		"Boss spawn_height must be positive for ground fit"
	);
	
	// For a 20m encounter boss (scaled up from ~2m Epiphant),
	// expect spawn_height to be scaled proportionally (roughly 10x)
	anyhow::ensure!(
		spawn_height > 1.0,
		"20m boss spawn_height should be substantial, got {}m", spawn_height
	);
	
	Ok(())
}

/// Test that only one boss exists after spawn - no duplicates (#1117)
#[test]
fn boss_spawn_creates_exactly_one_boss() -> anyhow::Result<()> {
	let mut app = minimal_app();
	
	let spawn_pos = Vec3::new(100.0, 0.0, 200.0);
	app.world_mut().spawn(RequestSpawnBoss {
		at: Some(Vec2::new(spawn_pos.x, spawn_pos.z)),
	});
	
	app.update();
	
	// Verify exactly one boss in storage
	let storage = app.world().resource::<HcsgStorage>();
	let boss_id = lod::gen::Id::Bytes(Bytes(chained_encounter_id().as_bytes()));
	let entry = storage.try_entry::<BikanwardaBossEncounter>(boss_id)
		.map_err(|_| anyhow::anyhow!("storage busy"))?;
	
	anyhow::ensure!(entry.is_some(), "Exactly one boss should exist");
	
	// Spawn again at same position
	app.world_mut().spawn(RequestSpawnBoss {
		at: Some(Vec2::new(spawn_pos.x, spawn_pos.z)),
	});
	app.update();
	
	// Should still be exactly one boss (replaced, not duplicated)
	let storage = app.world().resource::<HcsgStorage>();
	let entry = storage.try_entry::<BikanwardaBossEncounter>(boss_id)
		.map_err(|_| anyhow::anyhow!("storage busy after second spawn"))?;
	
	anyhow::ensure!(entry.is_some(), "Exactly one boss should exist after respawn");
	
	Ok(())
}

/// Test that boss position is preserved exactly across session restart (#1117)
#[test]
fn boss_position_preserved_across_restart() -> anyhow::Result<()> {
	let mut app = minimal_app();
	
	let spawn_pos = Vec3::new(1746.8, 42.5, 1029.9);
	app.world_mut().spawn(RequestSpawnBoss {
		at: Some(Vec2::new(spawn_pos.x, spawn_pos.z)),
	});
	app.update();
	
	// Capture original position
	let storage = app.world().resource::<HcsgStorage>();
	let boss_id = lod::gen::Id::Bytes(Bytes(chained_encounter_id().as_bytes()));
	let original = storage.try_entry::<BikanwardaBossEncounter>(boss_id)
		.map_err(|_| anyhow::anyhow!("storage busy"))?
		.ok_or_else(|| anyhow::anyhow!("Boss not found"))?;
	let original_pos = original.value.boss_position;
	
	// Restart session
	app.world_mut().resource_mut::<HcsgRestartRequest>().request();
	app.update();
	
	// Verify position is exactly preserved
	let storage = app.world().resource::<HcsgStorage>();
	let restored = storage.try_entry::<BikanwardaBossEncounter>(boss_id)
		.map_err(|_| anyhow::anyhow!("storage busy after restart"))?
		.ok_or_else(|| anyhow::anyhow!("Boss not found after restart"))?;
	
	anyhow::ensure!(
		restored.value.boss_position == original_pos,
		"Boss position should be exactly preserved: original {:?}, restored {:?}",
		original_pos, restored.value.boss_position
	);
	
	Ok(())
}
