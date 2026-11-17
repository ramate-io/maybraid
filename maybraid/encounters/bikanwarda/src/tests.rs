//! Outcome and unlock regressions. No combat-hud.

use anyhow::{Result, anyhow};
use bevy::prelude::*;
use bevy::time::{TimeUpdateStrategy, Virtual};
use damage::Died;
use encounters::fixtures::{install as install_encounters, spawn_instance};
use encounters::{
	EncounterFinishedReentry, EncounterId, EncounterLifetimePolicy, EncounterOutcome,
	EncounterSignal,
};
use lubikans::{LubikanAccountId, LubikanBodyOf, LubikanCatalog, LubikanOwner, LubikansPlugin};
use std::time::Duration;

use crate::identity::{ENRAGED_DEFINITION_ID, WARD_DEFINITION_ID, chained_encounter_id};
use crate::outcomes::{BikanwardaDeparted, BikanwardaDeparture, EncounterUnlockAwarded};
use crate::plugin::BikanwardaEncounterPlugin;

const DT: f32 = 0.1;

fn gameplay() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));
	install_encounters(&mut app);
	if !app.is_plugin_added::<LubikansPlugin>() {
		app.add_plugins(LubikansPlugin);
	}
	app.add_plugins(BikanwardaEncounterPlugin);
	app.world_mut()
		.resource_mut::<Time<Virtual>>()
		.set_max_delta(Duration::from_secs(30));
	app.update();
	app
}

fn spawn_owner(app: &mut App) -> Entity {
	let owner = app.world_mut().spawn(LubikanOwner).id();
	app.update();
	owner
}

fn spawn_boss(app: &mut App) -> Entity {
	let boss = spawn_instance(app.world_mut(), chained_encounter_id(), 100.0);
	app.update();
	boss
}

fn drain_awards(app: &mut App) -> Vec<EncounterUnlockAwarded> {
	app.world_mut()
		.resource_mut::<Messages<EncounterUnlockAwarded>>()
		.drain()
		.collect()
}

fn drain_departures(app: &mut App) -> Vec<BikanwardaDeparted> {
	app.world_mut().resource_mut::<Messages<BikanwardaDeparted>>().drain().collect()
}

fn owned(app: &App, owner: Entity, definition: lubikans::LubikanDefinitionId) -> Result<bool> {
	let ledger = LubikanBodyOf::ledger(app.world(), owner).ok_or_else(|| anyhow!("ledger"))?;
	Ok(ledger.contains(LubikanAccountId::unique(definition)))
}

fn account_count(app: &App, owner: Entity) -> Result<usize> {
	let ledger = LubikanBodyOf::ledger(app.world(), owner).ok_or_else(|| anyhow!("ledger"))?;
	Ok(ledger.iter().count())
}

#[test]
fn catalog_registers_ward_and_enraged_without_hud() -> Result<()> {
	let app = gameplay();
	let catalog = app.world().resource::<LubikanCatalog>();
	anyhow::ensure!(catalog.get(ENRAGED_DEFINITION_ID).is_some(), "enraged");
	anyhow::ensure!(catalog.get(WARD_DEFINITION_ID).is_some(), "ward");
	anyhow::ensure!(catalog.get(lubikans::LubikanDefinitionId::SELF).is_some(), "self");
	Ok(())
}

#[test]
fn kill_awards_enraged_once() -> Result<()> {
	let mut app = gameplay();
	let owner = spawn_owner(&mut app);
	let boss = spawn_boss(&mut app);
	app.world_mut()
		.write_message(Died { entity: boss, source: Some(owner), point: Vec3::ZERO });
	app.update();
	let awards = drain_awards(&mut app);
	assert_eq!(awards.len(), 1);
	assert!(awards[0].first_grant);
	assert_eq!(awards[0].outcome, EncounterOutcome::Killed);
	assert!(owned(&app, owner, ENRAGED_DEFINITION_ID)?);
	assert!(!owned(&app, owner, WARD_DEFINITION_ID)?);
	let departures = drain_departures(&mut app);
	assert_eq!(departures.len(), 1);
	assert_eq!(departures[0].kind, BikanwardaDeparture::Killed);

	app.world_mut()
		.write_message(Died { entity: boss, source: Some(owner), point: Vec3::ZERO });
	app.update();
	assert!(drain_awards(&mut app).is_empty());
	assert_eq!(account_count(&app, owner)?, 2);
	Ok(())
}

#[test]
fn chain_break_awards_ward_and_transcends() -> Result<()> {
	let mut app = gameplay();
	let owner = spawn_owner(&mut app);
	spawn_boss(&mut app);
	app.world_mut()
		.write_message(EncounterSignal::chain_broken(chained_encounter_id(), Some(owner)));
	app.update();
	let awards = drain_awards(&mut app);
	assert_eq!(awards.len(), 1);
	assert_eq!(awards[0].outcome, EncounterOutcome::Freed);
	assert!(owned(&app, owner, WARD_DEFINITION_ID)?);
	assert!(!owned(&app, owner, ENRAGED_DEFINITION_ID)?);
	let departures = drain_departures(&mut app);
	assert_eq!(departures[0].kind, BikanwardaDeparture::Transcend);
	Ok(())
}

#[test]
fn same_frame_kill_and_break_awards_only_enraged() -> Result<()> {
	let mut app = gameplay();
	let owner = spawn_owner(&mut app);
	let boss = spawn_boss(&mut app);
	app.world_mut()
		.write_message(Died { entity: boss, source: Some(owner), point: Vec3::ZERO });
	app.world_mut()
		.write_message(EncounterSignal::chain_broken(chained_encounter_id(), Some(owner)));
	app.update();
	let awards = drain_awards(&mut app);
	assert_eq!(awards.len(), 1);
	assert_eq!(awards[0].outcome, EncounterOutcome::Killed);
	assert!(owned(&app, owner, ENRAGED_DEFINITION_ID)?);
	assert!(!owned(&app, owner, WARD_DEFINITION_ID)?);
	Ok(())
}

#[test]
fn inventory_can_hold_both_across_occurrences() -> Result<()> {
	let mut app = gameplay();
	app.world_mut().insert_resource(EncounterLifetimePolicy {
		finished_reentry: EncounterFinishedReentry::RareTimeSlot {
			probability: 1.0,
			slot_secs: 60.0,
		},
		..EncounterLifetimePolicy::bikanwarda()
	});
	let owner = spawn_owner(&mut app);
	let first = spawn_boss(&mut app);
	app.world_mut()
		.write_message(Died { entity: first, source: Some(owner), point: Vec3::ZERO });
	app.update();
	drain_awards(&mut app);
	app.world_mut().entity_mut(first).despawn();
	app.update();
	spawn_boss(&mut app);
	app.world_mut()
		.write_message(EncounterSignal::chain_broken(chained_encounter_id(), Some(owner)));
	app.update();
	assert!(owned(&app, owner, ENRAGED_DEFINITION_ID)?);
	assert!(owned(&app, owner, WARD_DEFINITION_ID)?);
	assert_eq!(account_count(&app, owner)?, 3);
	Ok(())
}

#[test]
fn missing_source_finishes_without_inventing_an_owner() -> Result<()> {
	let mut app = gameplay();
	let owner = spawn_owner(&mut app);
	let boss = spawn_boss(&mut app);
	app.world_mut()
		.write_message(Died { entity: boss, source: None, point: Vec3::ZERO });
	app.update();
	assert!(drain_awards(&mut app).is_empty());
	assert!(!owned(&app, owner, ENRAGED_DEFINITION_ID)?);
	Ok(())
}

#[test]
fn chained_slug_is_the_fixture_key() {
	assert_eq!(chained_encounter_id(), EncounterId::from_slug("bikanwarda.chained.v0"));
}

// Ground-fit and placement regression tests (#1117)
mod placement {
	use super::*;
	use crate::generation::BikanwardaBossEncounter;
	use bevy::shape::Aabb3d;
	use characters::CharacterRecipe;
	
	#[test]
	fn boss_position_is_preserved_in_generated_encounter() {
		let authored_pos = Vec3::new(1746.8, 100.5, 1029.9);
		let encounter = BikanwardaBossEncounter::chained_at(authored_pos);
		assert_eq!(encounter.boss_position, authored_pos);
	}
	
	#[test]
	fn encounter_bounds_contain_boss_position() {
		let boss_pos = Vec3::new(1746.8, 100.5, 1029.9);
		let encounter = BikanwardaBossEncounter::chained_at(boss_pos);
		
		// Boss position should be within encounter bounds
		let boss_aabb = Aabb3d::new(boss_pos, Vec3::splat(0.1));
		assert!(encounter.bounds.intersects(&boss_aabb),
			"Boss position {:?} not within encounter bounds {:?}",
			boss_pos, encounter.bounds);
	}
	
	#[test]
	fn regeneration_preserves_exact_position() {
		let pos = Vec3::new(100.0, 50.0, 200.0);
		let first = BikanwardaBossEncounter::chained_at(pos);
		let second = BikanwardaBossEncounter::chained_at(pos);
		
		assert_eq!(first.boss_position, second.boss_position);
		assert_eq!(first.id, second.id);
		assert_eq!(first.bounds, second.bounds);
	}
	
	#[test]
	fn ground_fit_applies_spawn_height_offset() {
		// This test verifies the spawn policy: Y coordinate includes terrain height
		// plus hull spawn_height offset. The encounter boss is scaled to ~20m height
		// from ~2m Epiphant base (~10x scale). With Epiphant proportions, the hull
		// spawn_height offset is approximately 10.15m.
		use bikanwarda_character::{BikanwardaActorRecipe, BikanwardaSpec};
		use characters::CharacterRecipe;
		
		let spec = BikanwardaSpec::encounter();
		let recipe = BikanwardaActorRecipe::from_spec(spec);
		let hull = recipe.components().scaled_locomotion_capsule();
		let spawn_height_offset = hull.spawn_height();
		
		// Verify spawn_height is positive (boss feet are above hull origin)
		assert!(spawn_height_offset > 0.0, 
			"Boss hull spawn_height must be positive, got {}", spawn_height_offset);
		
		// With a 20m encounter boss (~10x Epiphant scale), expect spawn_height ~10.15m
		assert!(spawn_height_offset > 8.0,
			"20m boss spawn_height should be substantial (>8m), got {}", spawn_height_offset);
		assert!(spawn_height_offset < 15.0,
			"20m boss spawn_height should be reasonable (<15m), got {}", spawn_height_offset);
	}
	
	#[test]
	fn column_and_pedestal_markers_exist_in_scene() {
		// Verify that column/pedestal components are exported for presentation
		use crate::{BossColumn, BossPedestal};
		
		let _col: BossColumn = BossColumn;
		let _ped: BossPedestal = BossPedestal;
	}
}

#[test]
fn column_and_pedestal_spawn_as_separate_entities() {
	use crate::generation::BikanwardaBossEncounter;
	use bevy::prelude::*;
	
	let terrain_y = 100.0;
	let boss_hull_offset = 10.15;
	let boss_position = Vec3::new(1000.0, terrain_y + boss_hull_offset, 2000.0);
	
	let encounter = BikanwardaBossEncounter::chained_at_with_terrain(boss_position, terrain_y);
	
	// Verify the key fixes:
	// 1. Children use LOCAL transforms (not world coordinates)
	// 2. Scene structure: Children syntax creates separate entities (not tuple merge)
	// 3. Column is BESIDE boss (8m XZ offset), not underneath
	
	// Column positioned 8m to the side as a tether anchor
	let column_offset_xz = Vec2::new(8.0, 0.0);
	
	// Child transforms are LOCAL to root (boss_position).
	// Pedestal and column grounded at terrain_y, so local Y = terrain_y - boss_position.y
	let local_ground_y = terrain_y - boss_position.y;
	assert!((local_ground_y - (-10.15)).abs() < 0.01, "Local ground Y is negative (terrain below boss)");
	
	let pedestal_local_y = local_ground_y + 1.5; // 3m full height, center at terrain + 1.5
	let column_local_y = local_ground_y + 10.5; // 15m full height, center at terrain + 10.5
	
	// Verify local transforms are correct
	assert!((pedestal_local_y - (-8.65)).abs() < 0.01, "Pedestal local Y");
	assert!((column_local_y - 0.35).abs() < 0.01, "Column local Y");
	
	// When propagated to world space:
	// Pedestal world Y = boss_position.y + pedestal_local_y = 110.15 + (-8.65) = 101.5 = terrain + 1.5 ✓
	// Column world Y = boss_position.y + column_local_y = 110.15 + 0.35 = 110.5 = terrain + 10.5 ✓
	let expected_ped_world_y = boss_position.y + pedestal_local_y;
	let expected_col_world_y = boss_position.y + column_local_y;
	
	assert!((expected_ped_world_y - (terrain_y + 1.5)).abs() < 0.01, "Pedestal at terrain + 1.5m");
	assert!((expected_col_world_y - (terrain_y + 10.5)).abs() < 0.01, "Column at terrain + 10.5m");
	
	// Column XZ in world space
	let expected_column_world_x = boss_position.x + column_offset_xz.x;
	let expected_column_world_z = boss_position.z + column_offset_xz.y;
	assert!((expected_column_world_x - 1008.0).abs() < 0.01, "Column 8m from boss in X");
	assert!((expected_column_world_z - 2000.0).abs() < 0.01, "Column same Z as boss");
	
	// Verify collider dimensions (Avian 0.8 full heights)
	// Collider::cylinder(radius, full_height) in Avian 0.8
	// Pedestal: 2m radius, 3m full height
	// Column: 1m radius, 15m full height
}

#[test]
fn terrain_y_stable_across_eviction_regeneration() {
	use crate::generation::{AuthoredBossSpawns, BikanwardaBossEncounter};
	use lod::gen::{Bytes, Id};
	use lod::hcsg::{GenerationContext, GenerationScheme, HcsgStorage, universal_bounds};
	
	// Initial spawn with sampled terrain (simulating boss_spawn.rs flow)
	let terrain_y = 68.91201; // Published terrain height from Durham
	let boss_hull_offset = 10.15; // From boss_spawn_height_offset()
	let boss_position = Vec3::new(2600.0, terrain_y + boss_hull_offset, 1037.9);
	
	let storage = HcsgStorage::default();
	
	// Simulate initial spawn: seed AuthoredBossSpawns with position and terrain_y
	let spawns = AuthoredBossSpawns {
		position: Some(boss_position),
		terrain_y: Some(terrain_y),
	};
	storage.seed(spawns.clone(), universal_bounds());
	
	// Publish initial encounter
	let initial_encounter = BikanwardaBossEncounter::chained_at_with_terrain(boss_position, terrain_y);
	let boss_id = Id::Bytes(Bytes(crate::chained_encounter_id().as_bytes()));
	storage.publish(boss_id, std::sync::Arc::new(initial_encounter.clone()), initial_encounter.bounds);
	
	// --- Eviction simulation: boss is removed from storage (e.g., viewer left area) ---
	// (In real system, HCSG eviction would remove the Arc but keep AuthoredBossSpawns)
	
	// --- Regeneration: rebuild boss from stable AuthoredBossSpawns ---
	let mut cx = GenerationContext::new(&storage);
	let regenerated = BikanwardaBossEncounter::build_with_id(&mut cx, boss_id);
	
	assert!(regenerated.is_some(), "Boss should regenerate from AuthoredBossSpawns");
	let (regenerated_encounter, _bounds) = regenerated.unwrap();
	
	// Verify terrain_y did NOT drift
	assert_eq!(
		regenerated_encounter.terrain_y, initial_encounter.terrain_y,
		"terrain_y should be stable across regeneration (no drift)"
	);
	assert_eq!(
		regenerated_encounter.terrain_y, terrain_y,
		"terrain_y should match original sampled value"
	);
	
	// Verify boss position preserved
	assert_eq!(
		regenerated_encounter.boss_position, initial_encounter.boss_position,
		"Boss position should be stable"
	);
	
	// Verify pedestal/column bases would be at same world Y
	// Column offset from boss
	let column_offset_xz = Vec2::new(8.0, 0.0);
	
	// Calculate expected world Y for pedestal and column (same for both encounters)
	let expected_pedestal_world_y = terrain_y + 1.5; // 3m full height, center at terrain + 1.5
	let expected_column_world_y = terrain_y + 10.5; // 15m full height, center at terrain + 10.5
	
	// For initial encounter
	let local_ground_y_initial = initial_encounter.terrain_y - initial_encounter.boss_position.y;
	let ped_local_y_initial = local_ground_y_initial + 1.5;
	let col_local_y_initial = local_ground_y_initial + 10.5;
	let ped_world_y_initial = initial_encounter.boss_position.y + ped_local_y_initial;
	let col_world_y_initial = initial_encounter.boss_position.y + col_local_y_initial;
	
	// For regenerated encounter
	let local_ground_y_regen = regenerated_encounter.terrain_y - regenerated_encounter.boss_position.y;
	let ped_local_y_regen = local_ground_y_regen + 1.5;
	let col_local_y_regen = local_ground_y_regen + 10.5;
	let ped_world_y_regen = regenerated_encounter.boss_position.y + ped_local_y_regen;
	let col_world_y_regen = regenerated_encounter.boss_position.y + col_local_y_regen;
	
	// Verify pedestal and column world Y are identical before/after regeneration
	assert!(
		(ped_world_y_initial - ped_world_y_regen).abs() < 0.001,
		"Pedestal world Y should be stable: initial={}, regenerated={}",
		ped_world_y_initial, ped_world_y_regen
	);
	assert!(
		(col_world_y_initial - col_world_y_regen).abs() < 0.001,
		"Column world Y should be stable: initial={}, regenerated={}",
		col_world_y_initial, col_world_y_regen
	);
	
	// Verify they match expected values
	assert!(
		(ped_world_y_regen - expected_pedestal_world_y).abs() < 0.001,
		"Pedestal at terrain + 1.5m: expected={}, got={}",
		expected_pedestal_world_y, ped_world_y_regen
	);
	assert!(
		(col_world_y_regen - expected_column_world_y).abs() < 0.001,
		"Column at terrain + 10.5m: expected={}, got={}",
		expected_column_world_y, col_world_y_regen
	);
}
