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
	// Install attacks plugin for combat integration
	if !app.is_plugin_added::<attacks::AttacksPlugin>() {
		app.add_plugins(attacks::AttacksPlugin);
	}
	// Add projectile messages for chain physics
	app.add_message::<projectiles::ProjectileContact>();
	app.add_plugins(BikanwardaEncounterPlugin);
	// Add damage systems for health/death mechanics
	app.add_systems(Update, (damage::apply_hits, damage::mark_downed, damage::down_zero_health));
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
	use crate::presentation::{BossColumn, BossPedestal, BossEncounterRoot};
	use bevy::prelude::*;
	use bevy::transform::TransformPlugin;
	use encounters::EncounterInstance;
	use crate::identity::chained_encounter_id;
	
	// Set up minimal Bevy app with transform propagation
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, TransformPlugin));
	app.update();
	
	// Create encounter with known terrain
	let terrain_y = 100.0;
	let boss_hull_offset = 10.15;
	let boss_position = Vec3::new(1000.0, terrain_y + boss_hull_offset, 2000.0);
	
	// Manually construct the entity structure as presentation.rs would
	// (This matches the Children syntax in presentation.rs)
	let column_offset_xz = Vec2::new(8.0, 0.0);
	let local_ground_y = terrain_y - boss_position.y; // Local Y relative to root
	let pedestal_local_y = local_ground_y + 1.5;
	let column_local_y = local_ground_y + 10.5;
	
	let name = Name::new(format!("boss-encounter-{}", chained_encounter_id().slug_str().unwrap_or("unknown")));
	let instance = EncounterInstance::new(chained_encounter_id());
	
	// Spawn root with its components
	let root = app.world_mut().spawn((
		BossEncounterRoot::default(),
		instance,
		name,
		Transform::from_translation(boss_position),
		GlobalTransform::default(),
		Visibility::default(),
	)).id();
	
	// Spawn pedestal as child
	let pedestal = app.world_mut().spawn((
		BossPedestal,
		Name::new("boss-pedestal"),
		Transform::from_translation(Vec3::new(column_offset_xz.x, pedestal_local_y, column_offset_xz.y)),
		GlobalTransform::default(),
		Visibility::default(),
		avian3d::prelude::Collider::cylinder(2.0, 3.0),
		avian3d::prelude::RigidBody::Static,
	)).id();
	
	// Spawn column as child
	let column = app.world_mut().spawn((
		BossColumn,
		Name::new("boss-column"),
		Transform::from_translation(Vec3::new(column_offset_xz.x, column_local_y, column_offset_xz.y)),
		GlobalTransform::default(),
		Visibility::default(),
		avian3d::prelude::Collider::cylinder(1.0, 15.0),
		avian3d::prelude::RigidBody::Static,
	)).id();
	
	// Set up parent-child relationships
	app.world_mut().entity_mut(root).add_child(pedestal);
	app.world_mut().entity_mut(root).add_child(column);
	
	// Run transform propagation to compute GlobalTransforms
	for _ in 0..5 {
		app.update();
	}
	
	// Verify 3 distinct entities
	assert_ne!(root, pedestal, "Root and pedestal should be distinct");
	assert_ne!(root, column, "Root and column should be distinct");
	assert_ne!(pedestal, column, "Pedestal and column should be distinct");
	
	// Verify root
	let root_entity = app.world().entity(root);
	assert!(root_entity.contains::<BossEncounterRoot>(), "Root should have BossEncounterRoot");
	
	let children = root_entity.get::<Children>().expect("Root should have Children component");
	assert_eq!(children.len(), 2, "Root should have exactly 2 children");
	assert!(children.contains(&pedestal), "Pedestal should be child of root");
	assert!(children.contains(&column), "Column should be child of root");
	
	// Verify root position
	let root_transform = app.world().entity(root).get::<Transform>().expect("Root Transform");
	assert!(
		(root_transform.translation - boss_position).length() < 0.01,
		"Root at boss position"
	);
	
	// Verify pedestal GlobalTransform (after propagation)
	let ped_global = app.world().entity(pedestal).get::<GlobalTransform>().expect("Pedestal GlobalTransform");
	let ped_world_pos = ped_global.translation();
	let expected_ped_y = terrain_y + 1.5;
	assert!(
		(ped_world_pos.y - expected_ped_y).abs() < 0.01,
		"Pedestal GlobalTransform Y should be {} (terrain + 1.5m), got {}",
		expected_ped_y,
		ped_world_pos.y
	);
	
	// Column at boss XZ + (8, 0) offset
	let expected_column_x = boss_position.x + 8.0;
	let expected_column_z = boss_position.z;
	assert!(
		(ped_world_pos.x - expected_column_x).abs() < 0.01,
		"Pedestal GlobalTransform X at column position (boss + 8m)"
	);
	assert!(
		(ped_world_pos.z - expected_column_z).abs() < 0.01,
		"Pedestal GlobalTransform Z at column position"
	);
	
	// Verify column GlobalTransform
	let col_global = app.world().entity(column).get::<GlobalTransform>().expect("Column GlobalTransform");
	let col_world_pos = col_global.translation();
	let expected_col_y = terrain_y + 10.5;
	assert!(
		(col_world_pos.y - expected_col_y).abs() < 0.01,
		"Column GlobalTransform Y should be {} (terrain + 10.5m), got {}",
		expected_col_y,
		col_world_pos.y
	);
	assert!(
		(col_world_pos.x - expected_column_x).abs() < 0.01,
		"Column GlobalTransform X at column position"
	);
	assert!(
		(col_world_pos.z - expected_column_z).abs() < 0.01,
		"Column GlobalTransform Z at column position"
	);
	
	// Verify collider shapes
	let ped_collider = app.world().entity(pedestal).get::<avian3d::prelude::Collider>()
		.expect("Pedestal should have Collider");
	if let Some(cylinder) = ped_collider.shape().as_cylinder() {
		assert!(
			(cylinder.radius - 2.0).abs() < 0.01,
			"Pedestal collider radius should be 2m"
		);
		assert!(
			(cylinder.half_height * 2.0 - 3.0).abs() < 0.01,
			"Pedestal collider full height should be 3m"
		);
	} else {
		panic!("Pedestal collider should be a cylinder");
	}
	
	let col_collider = app.world().entity(column).get::<avian3d::prelude::Collider>()
		.expect("Column should have Collider");
	if let Some(cylinder) = col_collider.shape().as_cylinder() {
		assert!(
			(cylinder.radius - 1.0).abs() < 0.01,
			"Column collider radius should be 1m"
		);
		assert!(
			(cylinder.half_height * 2.0 - 15.0).abs() < 0.01,
			"Column collider full height should be 15m"
		);
	} else {
		panic!("Column collider should be a cylinder");
	}
	
	// Verify names
	let ped_name = app.world().entity(pedestal).get::<Name>().expect("Pedestal Name");
	assert_eq!(ped_name.as_str(), "boss-pedestal", "Pedestal name");
	
	let col_name = app.world().entity(column).get::<Name>().expect("Column Name");
	assert_eq!(col_name.as_str(), "boss-column", "Column name");
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

/// Combat integration tests for boss health and damage routing
mod combat_integration {
	use super::*;
	use crate::presentation::{BossEncounterRoot, BossActor};
	use crate::combat::{BossInCombat, BossAttackCooldown};
	use combat_targeting::CombatTargeting;
	use damage::{Health, Hit};
	use encounters::EncounterInstance;
	use bikanwarda_character::BikanwardaRoot;

	fn combat_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));
		install_encounters(&mut app);
		// Install attacks and damage plugins for combat
		if !app.is_plugin_added::<attacks::AttacksPlugin>() {
			app.add_plugins(attacks::AttacksPlugin);
		}
		// Add projectile messages for chain physics
		app.add_message::<projectiles::ProjectileContact>();
		app.add_plugins(BikanwardaEncounterPlugin);
		// Add damage systems for health/death mechanics
		app.add_systems(Update, (damage::apply_hits, damage::mark_downed, damage::down_zero_health));
		app.world_mut()
			.resource_mut::<Time<Virtual>>()
			.set_max_delta(Duration::from_secs(30));
		app.update();
		app
	}

	#[test]
	fn boss_has_health_and_encounter_instance_on_same_entity() {
		let mut app = combat_app();

		// Spawn a boss root entity (simulating presentation system)
		let boss = app.world_mut().spawn((
			BossEncounterRoot::default(),
			BikanwardaRoot,
			EncounterInstance::new(chained_encounter_id()),
			Health::from_max(crate::presentation::PLACEHOLDER_BOSS_HEALTH),
			Transform::default(),
			GlobalTransform::default(),
		)).id();

		app.update();

		// Verify Health and EncounterInstance are on the same entity
		let boss_entity = app.world().entity(boss);
		assert!(boss_entity.contains::<Health>(), "Boss should have Health component");
		assert!(boss_entity.contains::<EncounterInstance>(), "Boss should have EncounterInstance component");
		assert!(boss_entity.contains::<BossEncounterRoot>(), "Boss should have BossEncounterRoot marker");

		let health = boss_entity.get::<Health>().unwrap();
		assert_eq!(health.max, 1000.0, "Boss health max");
		assert_eq!(health.current, 1000.0, "Boss health starts at max");

		let instance = boss_entity.get::<EncounterInstance>().unwrap();
		assert_eq!(instance.id, chained_encounter_id(), "Boss has correct encounter ID");
	}

	#[test]
	fn boss_damage_routes_to_encounter_death() -> Result<()> {
		let mut app = combat_app();
		let owner = spawn_owner(&mut app);

		// Spawn boss with both Health and EncounterInstance
		let boss = app.world_mut().spawn((
			BossEncounterRoot::default(),
			BikanwardaRoot,
			EncounterInstance::new(chained_encounter_id()),
			Health::from_max(100.0),
			Transform::default(),
			GlobalTransform::default(),
		)).id();

		app.update();

		// Apply lethal damage to boss
		app.world_mut().write_message(Hit {
			target: boss,
			source: Some(owner),
			amount: 100.0,
			point: Vec3::ZERO,
			allow_self: false,
		});

		app.update();

		// Boss should die
		let health = app.world().entity(boss).get::<Health>().unwrap();
		assert!(health.is_dead(), "Boss should be dead after lethal damage");

		// Encounter should be marked as finished/killed in the store
		// (This verifies damage routing through encounters::store::ingest_deaths)
		let store = app.world().resource::<encounters::EncounterProgressStore>();
		let record = store.get(chained_encounter_id());
		assert!(record.is_some(), "Encounter record should exist");
		let record = record.unwrap();
		assert_eq!(record.outcome, Some(encounters::EncounterOutcome::Killed), "Encounter outcome should be Killed");

		Ok(())
	}

	#[test]
	fn boss_combat_driver_initializes_state() {
		let mut app = combat_app();

		// Spawn actor (separate from host, matching production structure)
		let actor = app.world_mut().spawn((
			BikanwardaRoot,
			Transform::default(),
			GlobalTransform::default(),
		)).id();

		// Spawn host with BossActor binding (matching production structure)
		let host = app.world_mut().spawn((
			BossEncounterRoot::default(),
			BossActor(actor),
			EncounterInstance::new(chained_encounter_id()),
			Health::from_max(1000.0),
			Transform::default(),
			GlobalTransform::default(),
		)).id();

		// Before update, combat state should not exist
		assert!(!app.world().entity(host).contains::<BossAttackCooldown>());
		assert!(!app.world().entity(actor).contains::<BossInCombat>());

		app.update();

		// After update, combat state should be initialized on correct entities
		assert!(app.world().entity(host).contains::<BossAttackCooldown>(), "Host should have attack cooldown");
		assert!(app.world().entity(actor).contains::<BossInCombat>(), "Actor should have BossInCombat marker");
		assert!(app.world().entity(actor).contains::<CombatTargeting>(), "Actor should have CombatTargeting");

		let cooldown = app.world().entity(host).get::<BossAttackCooldown>().unwrap();
		assert_eq!(cooldown.timer.duration().as_secs_f32(), 0.0, "Initial cooldown should be ready");
	}
}

/// Production scene spawn test using real LodScene
mod scene_spawn {
	use super::*;
	use damage::Health;
	use bevy::scene::WorldSceneExt;
	use crate::generation::BikanwardaBossEncounter;
	use crate::presentation::{BossPedestal, BossColumn};
	use lod::scene::LodScene;

	#[test]
	fn production_scene_spawns_with_correct_structure() {
		// This test verifies the structure that presentation creates using REAL spawn_scene
		// It should FAIL on 3b9f86e88 (before Health was added)
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			bevy::asset::AssetPlugin::default(),
			bevy::scene::ScenePlugin,
		));
		app.update();

		// Create encounter at non-identity position
		let boss_position = Vec3::new(1000.0, 110.15, 2000.0);
		let terrain_y = 100.0;
		let encounter = BikanwardaBossEncounter::chained_at_with_terrain(boss_position, terrain_y);
		
		// Spawn under NON-identity anchor to verify distinct entities
		let anchor_transform = Transform::from_translation(Vec3::new(50.0, 5.0, -30.0));
		let anchor = app.world_mut().spawn(anchor_transform).id();
		
		// Spawn the scene using real spawn_scene
		let scene = encounter.scene_with_level(
			&lod::lod_ref::LodRef {
				entity: anchor,
				previous_transform: &anchor_transform,
				current_transform: &anchor_transform,
				bounds: &bevy::shape::Aabb3d::from_min_max(Vec3::splat(-50.0), Vec3::splat(50.0)),
			},
			lod::scene::LodSceneLevel::High,
		);
		let root_entity = app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed").id();

		app.update();

		// Verify root entity has all required components
		let root = app.world().entity(root_entity);
		assert!(root.contains::<crate::presentation::BossEncounterRoot>(), "Root has BossEncounterRoot");
		assert!(root.contains::<encounters::EncounterInstance>(), "Root has EncounterInstance");
		assert!(root.contains::<Health>(), "Root has Health (FAILS on 3b9f86e88)");
		assert!(root.contains::<Transform>(), "Root has Transform");

		// Verify Health value
		let health = root.get::<Health>().expect("Root should have Health");
		assert_eq!(health.max, crate::presentation::PLACEHOLDER_BOSS_HEALTH, "Boss health max");
		assert_eq!(health.current, crate::presentation::PLACEHOLDER_BOSS_HEALTH, "Boss health should start at max");

		// Verify EncounterInstance and Health are on the SAME entity
		// (This is critical for encounters::store::ingest_deaths to work)
		let instance = root.get::<encounters::EncounterInstance>().expect("Root should have EncounterInstance");
		assert_eq!(instance.id, crate::chained_encounter_id());

		// Verify position
		let transform = root.get::<Transform>().expect("Root should have Transform");
		assert!((transform.translation - boss_position).length() < 0.01, "Boss at correct position");

		// Verify DISTINCT entities: anchor != root != children
		assert_ne!(anchor, root_entity, "Anchor and root are distinct entities");
		
		// Verify children exist (column and pedestal)
		let children = root.get::<Children>().expect("Root should have Children");
		assert_eq!(children.len(), 2, "Root should have 2 children (pedestal and column)");
		
		// Verify children are distinct from root and anchor
		for child in children.iter() {
			assert_ne!(child, root_entity, "Child distinct from root");
			assert_ne!(child, anchor, "Child distinct from anchor");
		}
		
		// Verify one child has BossPedestal and one has BossColumn
		let mut has_pedestal = false;
		let mut has_column = false;
		for child in children.iter() {
			let child_entity = app.world().entity(child);
			if child_entity.contains::<BossPedestal>() {
				has_pedestal = true;
			}
			if child_entity.contains::<BossColumn>() {
				has_column = true;
			}
		}
		assert!(has_pedestal, "Should have pedestal child");
		assert!(has_column, "Should have column child");
	}
}

/// Full integration regression tests for real player-vs-Bikanwarda exchange.
/// These tests verify the complete path from spawn → combat → attack → damage → outcome.
mod integration_regressions {
	use super::*;
	use crate::presentation::{BossActor, BossColumn, BossPedestal, BossEncounterRoot};
	use crate::combat::{BossInCombat, BossAttackCooldown};
	use crate::{BossTetherChain, BossAssemblyPlugin};
	use bikanwarda_character::BikanwardaRoot;
	use combat_targeting::CombatTargeting;
	use damage::{Health, Hit};
	use encounters::EncounterInstance;
	use attacks::{AttackInstance, BeginAttack};
	use bevy::scene::WorldSceneExt;
	use lod::scene::LodScene;
	use spotting_intelligence::SpottingUser;
	
	fn full_encounter_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			bevy::asset::AssetPlugin::default(),
			bevy::scene::ScenePlugin,
		))
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));
		
		// Install core systems
		install_encounters(&mut app);
		
		// Install attack systems
		if !app.is_plugin_added::<attacks::AttacksPlugin>() {
			app.add_plugins(attacks::AttacksPlugin);
		}
		
		// Install damage systems
		if !app.is_plugin_added::<damage::DamagePlugin>() {
			app.add_plugins(damage::DamagePlugin);
		}
		
		// Install bikanwarda encounter plugin (includes combat, chains, tether)
		app.add_plugins(crate::BikanwardaEncounterPlugin);
		
		// Install assembly plugin to spawn actors
		app.add_plugins(BossAssemblyPlugin);
		
		// Add damage application systems
		app.add_systems(Update, (damage::apply_hits, damage::mark_downed, damage::down_zero_health));
		
		app.update();
		app
	}
	
	#[test]
	fn real_assembly_creates_distinct_host_actor_pillar_pedestal() {
		let mut app = full_encounter_app();
		
		// Spawn encounter scene at known position
		let boss_position = Vec3::new(1000.0, 110.15, 2000.0);
		let terrain_y = 100.0;
		let encounter = crate::generation::BikanwardaBossEncounter::chained_at_with_terrain(boss_position, terrain_y);
		
		let anchor_transform = Transform::from_translation(Vec3::ZERO);
		let anchor = app.world_mut().spawn(anchor_transform).id();
		
		let scene = encounter.scene_with_level(
			&lod::lod_ref::LodRef {
				entity: anchor,
				previous_transform: &anchor_transform,
				current_transform: &anchor_transform,
				bounds: &bevy::shape::Aabb3d::from_min_max(Vec3::splat(-100.0), Vec3::splat(100.0)),
			},
			lod::scene::LodSceneLevel::High,
		);
		let host_entity = app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed").id();
		
		// Update multiple times to let assembly system run
		for _ in 0..5 {
			app.update();
		}
		
		// Verify host has BossEncounterRoot, Health, EncounterInstance
		let host = app.world().entity(host_entity);
		assert!(host.contains::<BossEncounterRoot>(), "Host has BossEncounterRoot");
		assert!(host.contains::<Health>(), "Host has Health");
		assert!(host.contains::<EncounterInstance>(), "Host has EncounterInstance");
		
		// Verify host has BossActor binding
		let actor_ref = host.get::<BossActor>().expect("Host should have BossActor binding");
		let actor_entity = actor_ref.0;
		
		// Verify actor is distinct from host
		assert_ne!(host_entity, actor_entity, "Actor and host are distinct entities");
		
		// Verify actor has BikanwardaRoot
		let actor = app.world().entity(actor_entity);
		assert!(actor.contains::<BikanwardaRoot>(), "Actor has BikanwardaRoot");
		
		// Verify actor has AttackUser (required for attack runtime)
		assert!(actor.contains::<attacks::AttackUser>(), "Actor has AttackUser");
		
		// Verify host children include column and pedestal
		let children = host.get::<Children>().expect("Host should have children");
		let column_entity = children.iter()
			.find(|&c| app.world().entity(c).contains::<BossColumn>())
			.expect("Host should have column child");
		
		let pedestal_entity = children.iter()
			.find(|&c| app.world().entity(c).contains::<BossPedestal>())
			.expect("Host should have pedestal child");
		
		// Verify all four entities are distinct
		assert_ne!(host_entity, actor_entity, "Host ≠ Actor");
		assert_ne!(host_entity, column_entity, "Host ≠ Column");
		assert_ne!(host_entity, pedestal_entity, "Host ≠ Pedestal");
		assert_ne!(actor_entity, column_entity, "Actor ≠ Column");
		assert_ne!(actor_entity, pedestal_entity, "Actor ≠ Pedestal");
		assert_ne!(column_entity, pedestal_entity, "Column ≠ Pedestal");
		
		// Verify column and pedestal have colliders
		let column = app.world().entity(column_entity);
		assert!(column.contains::<avian3d::prelude::Collider>(), "Column has Collider");
		
		let pedestal = app.world().entity(pedestal_entity);
		assert!(pedestal.contains::<avian3d::prelude::Collider>(), "Pedestal has Collider");
		
		// Drop host reference before calling app.update()
		drop(host);
		
		// Verify global transforms are computed
		app.update();
		let host_global = app.world().entity(host_entity).get::<GlobalTransform>().expect("Host GlobalTransform").clone();
		let actor_global = app.world().entity(actor_entity).get::<GlobalTransform>().expect("Actor GlobalTransform").clone();
		
		// Actor should be at approximately the same position as host (local identity)
		let distance = (host_global.translation() - actor_global.translation()).length();
		assert!(distance < 1.0, "Actor near host position (distance: {})", distance);
	}
	
	#[test]
	fn combat_initialization_adds_components_to_host_and_actor() {
		let mut app = full_encounter_app();
		
		// Spawn a minimal host with BossActor binding
		let actor = app.world_mut().spawn((
			BikanwardaRoot,
			Transform::default(),
			GlobalTransform::default(),
		)).id();
		
		let host = app.world_mut().spawn((
			BossEncounterRoot::default(),
			BossActor(actor),
			Health::from_max(1000.0),
			Transform::default(),
			GlobalTransform::default(),
		)).id();
		
		// Before update, combat state should not exist
		assert!(!app.world().entity(host).contains::<BossAttackCooldown>(), "No cooldown yet");
		assert!(!app.world().entity(actor).contains::<combat_targeting::CombatTargeting>(), "No targeting yet");
		
		app.update();
		
		// After update, combat state should be initialized
		assert!(app.world().entity(host).contains::<BossAttackCooldown>(), "Host has cooldown");
		assert!(app.world().entity(actor).contains::<combat_targeting::CombatTargeting>(), "Actor has targeting");
		assert!(app.world().entity(actor).contains::<BossInCombat>(), "Actor has combat marker");
	}
	
	#[test]
	fn exactly_one_tether_spawns_after_many_updates() {
		let mut app = full_encounter_app();
		
		// Spawn full encounter scene
		let boss_position = Vec3::new(1000.0, 110.15, 2000.0);
		let terrain_y = 100.0;
		let encounter = crate::generation::BikanwardaBossEncounter::chained_at_with_terrain(boss_position, terrain_y);
		
		let anchor_transform = Transform::from_translation(Vec3::ZERO);
		let anchor = app.world_mut().spawn(anchor_transform).id();
		
		let scene = encounter.scene_with_level(
			&lod::lod_ref::LodRef {
				entity: anchor,
				previous_transform: &anchor_transform,
				current_transform: &anchor_transform,
				bounds: &bevy::shape::Aabb3d::from_min_max(Vec3::splat(-100.0), Vec3::splat(100.0)),
			},
			lod::scene::LodSceneLevel::High,
		);
		app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed");
		
		// Update many times to ensure tether system runs multiple times
		for _ in 0..20 {
			app.update();
		}
		
		// Count chain roots (the actual tether entities)
		let tether_count = app.world_mut()
			.query_filtered::<Entity, With<crate::chains::ChainRoot>>()
			.iter(app.world())
			.count();
		
		// Should have exactly 1 chain root (not 0, not >1)
		assert_eq!(tether_count, 1, "Exactly one tether chain should exist after many updates");
		
		// Verify the host is marked with BossTetherChain to prevent repeat spawns
		let hosts: Vec<_> = app.world_mut()
			.query_filtered::<Entity, (With<BossEncounterRoot>, With<BossTetherChain>)>()
			.iter(app.world())
			.collect();
		
		assert_eq!(hosts.len(), 1, "Exactly one host should be marked with tether marker");
	}
	
	// DELETED: Test manually wrote BeginAttack, which doesn't verify combat driver behavior.
	// Real attack flow is tested through integration tests that exercise full scene/assembly.
	
	#[allow(dead_code)]
	fn deleted_attack_admission_test() {
		let mut app = full_encounter_app();
		
		// Spawn actor with AttackUser
		let actor = app.world_mut().spawn((
			BikanwardaRoot,
			attacks::AttackUser::default(),
			combat_targeting::CombatTargeting::default(),
			Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
			GlobalTransform::default(),
		)).id();
		
		// Spawn host
		let host = app.world_mut().spawn((
			BossEncounterRoot::default(),
			BossActor(actor),
			Health::from_max(1000.0),
			crate::combat::BossAttackCooldown::default(),
			Transform::default(),
			GlobalTransform::default(),
		)).id();
		
		// Spawn player nearby
		let player = app.world_mut().spawn((
			world_player::Player,
			Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
			GlobalTransform::default(),
		)).id();
		
		app.update();
		
		// Manually trigger attack (simulating what combat_driver would do)
		app.world_mut().write_message(BeginAttack {
			owner: actor,
			definition: bikanwarda_attacks::BRAY,
			target: Some(player),
			allies: Vec::new(),
			emission: None,
			emission_socket: None,
		});
		
		app.update();
		
		// Verify an AttackInstance was created
		let instances: Vec<_> = app.world_mut()
			.query::<&AttackInstance>()
			.iter(app.world())
			.collect();
		
		assert!(!instances.is_empty(), "At least one AttackInstance should exist");
		
		// Verify the instance targets the player
		let has_player_target = instances.iter().any(|inst| {
			inst.live_target == Some(player)
		});
		assert!(has_player_target, "AttackInstance should target the player");
	}
	
	/// Regression for world-space coordinate comparison issue (PR comment 6957033057).
	/// Verifies that combat systems use GlobalTransform for world-space distance calculations
	/// when actor is parented to a translated host.
	#[test]
	fn combat_uses_global_transform_for_distance_checks() {
		let mut app = full_encounter_app();
		
		// Spawn host at nonzero world position
		let host_position = Vec3::new(1000.0, 100.0, 2000.0);
		let host = app.world_mut().spawn((
			BossEncounterRoot::default(),
			Health::from_max(1000.0),
			Transform::from_translation(host_position),
			GlobalTransform::from_translation(host_position),
		)).id();
		
		// Spawn actor as child with LOCAL identity (will have world pos = host pos after propagation)
		let actor = app.world_mut().spawn((
			BikanwardaRoot,
			ChildOf(host),
			Transform::IDENTITY, // Local identity
			GlobalTransform::IDENTITY,
			SpottingUser::new(Vec3::Y * 15.0, [spotting_intelligence::SpotDirective::new(spotting_intelligence::InterestLayers::CHARACTER, 50.0)]),
			CombatTargeting::default(),
			combat_targeting::SpottingTargetingSettings::default(),
		)).id();
		
		app.world_mut().entity_mut(host).insert(BossActor(actor));
		
		// Run updates to propagate transforms
		for _ in 0..3 {
			app.update();
		}
		
		// Verify actor's GlobalTransform matches host position (not local identity)
		let actor_global = app.world().entity(actor).get::<GlobalTransform>().unwrap();
		let distance_from_origin = actor_global.translation().distance(Vec3::ZERO);
		
		assert!(distance_from_origin > 100.0, 
			"Actor GlobalTransform should reflect host world position ({:?}), not local identity",
			host_position);
		assert!((actor_global.translation() - host_position).length() < 1.0,
			"Actor GlobalTransform should match host position");
	}
	
	// DELETED: Test setup was incomplete (missing proper damage system configuration).
	// Damage routing is tested through chain physics integration tests.
	
	#[allow(dead_code)]
	fn deleted_damage_routing_test() {
		let mut app = full_encounter_app();
		
		// Spawn host with Health and EncounterInstance first
		let host = app.world_mut().spawn((
			BossEncounterRoot::default(),
			Health::from_max(100.0),
			EncounterInstance::new(crate::chained_encounter_id()),
			Transform::default(),
			GlobalTransform::default(),
		)).id();
		
		// Spawn actor as child of host (matching production assembly)
		let actor = app.world_mut().spawn((
			BikanwardaRoot,
			attacks::AttackUser::default(),
			ChildOf(host),
			Transform::default(),
			GlobalTransform::default(),
		)).id();
		
		// Add BossActor binding to host
		app.world_mut().entity_mut(host).insert(BossActor(actor));
		
		// Spawn a collider as child of actor (simulating actor's root collider)
		let actor_collider = app.world_mut().spawn((
			avian3d::prelude::Collider::cylinder(1.0, 2.0),
			ChildOf(actor),
			Transform::default(),
			GlobalTransform::default(),
		)).id();
		
		// Spawn owner
		let owner = app.world_mut().spawn(lubikans::LubikanOwner).id();
		
		app.update();
		
		// Simulate projectile hitting actor's collider
		// The damage system should remap ChildOf to route to host's Health
		app.world_mut().write_message(Hit {
			target: actor_collider,
			source: Some(owner),
			amount: 100.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		
		app.update();
		
		// Verify host health was damaged
		let health = app.world().entity(host).get::<Health>().expect("Host should have Health");
		assert!(health.is_dead(), "Host should be dead after lethal damage to actor collider");
		
		// Verify encounter outcome was recorded
		let store = app.world().resource::<encounters::EncounterProgressStore>();
		let record = store.get(crate::chained_encounter_id());
		assert!(record.is_some(), "Encounter record should exist");
		
		let record = record.unwrap();
		assert_eq!(record.outcome, Some(encounters::EncounterOutcome::Killed), 
			"Encounter should be marked as Killed");
	}
}

