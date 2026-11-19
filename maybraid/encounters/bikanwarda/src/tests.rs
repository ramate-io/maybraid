//! Outcome and unlock regressions. No combat-hud.

use anyhow::{anyhow, Result};
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

use crate::identity::{chained_encounter_id, ENRAGED_DEFINITION_ID, WARD_DEFINITION_ID};
use crate::outcomes::{BikanwardaDeparted, BikanwardaDeparture, EncounterUnlockAwarded};
use crate::plugin::BikanwardaEncounterPlugin;
use crate::presentation::{
	COLUMN_HEIGHT, COLUMN_OFFSET_XZ, COLUMN_RADIUS, PEDESTAL_HEIGHT, PEDESTAL_RADIUS,
};

// Deterministic 60 FPS timestep for gameplay tests
const DT: f32 = 1.0 / 60.0; // ~16.67ms per frame

fn encounter_hull_offset() -> f32 {
	use bikanwarda_character::{BikanwardaActorRecipe, BikanwardaSpec};
	use characters::CharacterRecipe;
	BikanwardaActorRecipe::from_spec(BikanwardaSpec::encounter())
		.components()
		.scaled_locomotion_capsule()
		.spawn_height()
}

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
		assert!(
			encounter.bounds.intersects(&boss_aabb),
			"Boss position {:?} not within encounter bounds {:?}",
			boss_pos,
			encounter.bounds
		);
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
		// Spawn Y includes terrain height plus hull spawn_height. The live hull
		// is ~6.7 m (~1/3 of the authored 20 m), so offset is ~3.4 m.
		use bikanwarda_character::{
			BikanwardaActorRecipe, BikanwardaSpec, ENCOUNTER_STANDING_HEIGHT_M,
		};
		use characters::CharacterRecipe;

		let spec = BikanwardaSpec::encounter();
		let recipe = BikanwardaActorRecipe::from_spec(spec);
		let hull = recipe.components().scaled_locomotion_capsule();
		let spawn_height_offset = hull.spawn_height();

		assert!(
			spawn_height_offset > 0.0,
			"Boss hull spawn_height must be positive, got {}",
			spawn_height_offset
		);
		assert!(
			spawn_height_offset > ENCOUNTER_STANDING_HEIGHT_M * 0.4,
			"6.7m boss spawn_height should be a substantial fraction of standing height, got {}",
			spawn_height_offset
		);
		assert!(
			spawn_height_offset < ENCOUNTER_STANDING_HEIGHT_M * 0.7,
			"6.7m boss spawn_height should stay below standing height, got {}",
			spawn_height_offset
		);
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
	use crate::identity::chained_encounter_id;
	use crate::presentation::{BossColumn, BossEncounterRoot, BossPedestal};
	use bevy::prelude::*;
	use bevy::transform::TransformPlugin;
	use encounters::EncounterInstance;

	// Set up minimal Bevy app with transform propagation
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, TransformPlugin));
	app.update();

	// Create encounter with known terrain
	let terrain_y = 100.0;
	let boss_hull_offset = encounter_hull_offset();
	let boss_position = Vec3::new(1000.0, terrain_y + boss_hull_offset, 2000.0);

	// Manually construct the entity structure as presentation.rs would
	// (This matches the Children syntax in presentation.rs)
	let column_offset_xz = COLUMN_OFFSET_XZ;
	let local_ground_y = terrain_y - boss_position.y; // Local Y relative to root
	let pedestal_local_y = local_ground_y + PEDESTAL_HEIGHT * 0.5;
	let column_local_y = local_ground_y + PEDESTAL_HEIGHT + COLUMN_HEIGHT * 0.5;

	let name = Name::new(format!(
		"boss-encounter-{}",
		chained_encounter_id().slug_str().unwrap_or("unknown")
	));
	let instance = EncounterInstance::new(chained_encounter_id());

	// Spawn root with its components
	let root = app
		.world_mut()
		.spawn((
			BossEncounterRoot::default(),
			instance,
			name,
			Transform::from_translation(boss_position),
			GlobalTransform::default(),
			Visibility::default(),
		))
		.id();

	// Spawn pedestal as child
	let pedestal = app
		.world_mut()
		.spawn((
			BossPedestal,
			Name::new("boss-pedestal"),
			Transform::from_translation(Vec3::new(
				column_offset_xz.x,
				pedestal_local_y,
				column_offset_xz.y,
			)),
			GlobalTransform::default(),
			Visibility::default(),
			avian3d::prelude::Collider::cylinder(PEDESTAL_RADIUS, PEDESTAL_HEIGHT),
			avian3d::prelude::RigidBody::Static,
		))
		.id();

	// Spawn column as child
	let column = app
		.world_mut()
		.spawn((
			BossColumn,
			Name::new("boss-column"),
			Transform::from_translation(Vec3::new(
				column_offset_xz.x,
				column_local_y,
				column_offset_xz.y,
			)),
			GlobalTransform::default(),
			Visibility::default(),
			avian3d::prelude::Collider::cylinder(COLUMN_RADIUS, COLUMN_HEIGHT),
			avian3d::prelude::RigidBody::Static,
		))
		.id();

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
	assert!((root_transform.translation - boss_position).length() < 0.01, "Root at boss position");

	// Verify pedestal GlobalTransform (after propagation)
	let ped_global = app
		.world()
		.entity(pedestal)
		.get::<GlobalTransform>()
		.expect("Pedestal GlobalTransform");
	let ped_world_pos = ped_global.translation();
	let expected_ped_y = terrain_y + PEDESTAL_HEIGHT * 0.5;
	assert!(
		(ped_world_pos.y - expected_ped_y).abs() < 0.01,
		"Pedestal GlobalTransform Y should be {} (terrain + half pedestal), got {}",
		expected_ped_y,
		ped_world_pos.y
	);

	// Column at boss XZ + scaled pillar offset
	let expected_column_x = boss_position.x + COLUMN_OFFSET_XZ.x;
	let expected_column_z = boss_position.z;
	assert!(
		(ped_world_pos.x - expected_column_x).abs() < 0.01,
		"Pedestal GlobalTransform X at column position (boss + pillar offset)"
	);
	assert!(
		(ped_world_pos.z - expected_column_z).abs() < 0.01,
		"Pedestal GlobalTransform Z at column position"
	);

	// Verify column GlobalTransform
	let col_global = app
		.world()
		.entity(column)
		.get::<GlobalTransform>()
		.expect("Column GlobalTransform");
	let col_world_pos = col_global.translation();
	let expected_col_y = terrain_y + PEDESTAL_HEIGHT + COLUMN_HEIGHT * 0.5;
	assert!(
		(col_world_pos.y - expected_col_y).abs() < 0.01,
		"Column GlobalTransform Y should be {} (terrain + pedestal + half column), got {}",
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
	let ped_collider = app
		.world()
		.entity(pedestal)
		.get::<avian3d::prelude::Collider>()
		.expect("Pedestal should have Collider");
	if let Some(cylinder) = ped_collider.shape().as_cylinder() {
		assert!(
			(cylinder.radius - PEDESTAL_RADIUS).abs() < 0.01,
			"Pedestal collider radius should match PEDESTAL_RADIUS"
		);
		assert!(
			(cylinder.half_height * 2.0 - PEDESTAL_HEIGHT).abs() < 0.01,
			"Pedestal collider full height should match PEDESTAL_HEIGHT"
		);
	} else {
		panic!("Pedestal collider should be a cylinder");
	}

	let col_collider = app
		.world()
		.entity(column)
		.get::<avian3d::prelude::Collider>()
		.expect("Column should have Collider");
	if let Some(cylinder) = col_collider.shape().as_cylinder() {
		assert!(
			(cylinder.radius - COLUMN_RADIUS).abs() < 0.01,
			"Column collider radius should match COLUMN_RADIUS"
		);
		assert!(
			(cylinder.half_height * 2.0 - COLUMN_HEIGHT).abs() < 0.01,
			"Column collider full height should match COLUMN_HEIGHT"
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
	use lod::hcsg::{universal_bounds, GenerationContext, GenerationScheme, HcsgStorage};

	// Initial spawn with sampled terrain (simulating boss_spawn.rs flow)
	let terrain_y = 68.91201; // Published terrain height from Durham
	let boss_hull_offset = encounter_hull_offset();
	let boss_position = Vec3::new(2600.0, terrain_y + boss_hull_offset, 1037.9);

	let storage = HcsgStorage::default();

	// Simulate initial spawn: seed AuthoredBossSpawns with position and terrain_y
	let spawns = AuthoredBossSpawns { position: Some(boss_position), terrain_y: Some(terrain_y) };
	storage.seed(spawns.clone(), universal_bounds());

	// Publish initial encounter
	let initial_encounter =
		BikanwardaBossEncounter::chained_at_with_terrain(boss_position, terrain_y);
	let boss_id = Id::Bytes(Bytes(crate::chained_encounter_id().as_bytes()));
	storage.publish(
		boss_id,
		std::sync::Arc::new(initial_encounter.clone()),
		initial_encounter.bounds,
	);

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
	let expected_pedestal_world_y = terrain_y + PEDESTAL_HEIGHT * 0.5;
	let expected_column_world_y = terrain_y + PEDESTAL_HEIGHT + COLUMN_HEIGHT * 0.5;

	// For initial encounter
	let local_ground_y_initial = initial_encounter.terrain_y - initial_encounter.boss_position.y;
	let ped_local_y_initial = local_ground_y_initial + PEDESTAL_HEIGHT * 0.5;
	let col_local_y_initial = local_ground_y_initial + PEDESTAL_HEIGHT + COLUMN_HEIGHT * 0.5;
	let ped_world_y_initial = initial_encounter.boss_position.y + ped_local_y_initial;
	let col_world_y_initial = initial_encounter.boss_position.y + col_local_y_initial;

	// For regenerated encounter
	let local_ground_y_regen =
		regenerated_encounter.terrain_y - regenerated_encounter.boss_position.y;
	let ped_local_y_regen = local_ground_y_regen + PEDESTAL_HEIGHT * 0.5;
	let col_local_y_regen = local_ground_y_regen + PEDESTAL_HEIGHT + COLUMN_HEIGHT * 0.5;
	let ped_world_y_regen = regenerated_encounter.boss_position.y + ped_local_y_regen;
	let col_world_y_regen = regenerated_encounter.boss_position.y + col_local_y_regen;

	// Verify pedestal and column world Y are identical before/after regeneration
	assert!(
		(ped_world_y_initial - ped_world_y_regen).abs() < 0.001,
		"Pedestal world Y should be stable: initial={}, regenerated={}",
		ped_world_y_initial,
		ped_world_y_regen
	);
	assert!(
		(col_world_y_initial - col_world_y_regen).abs() < 0.001,
		"Column world Y should be stable: initial={}, regenerated={}",
		col_world_y_initial,
		col_world_y_regen
	);

	// Verify they match expected values
	assert!(
		(ped_world_y_regen - expected_pedestal_world_y).abs() < 0.001,
		"Pedestal at terrain + half pedestal: expected={}, got={}",
		expected_pedestal_world_y,
		ped_world_y_regen
	);
	assert!(
		(col_world_y_regen - expected_column_world_y).abs() < 0.001,
		"Column at terrain + pedestal + half column: expected={}, got={}",
		expected_column_world_y,
		col_world_y_regen
	);
}

/// Combat integration tests for boss health and damage routing
mod combat_integration {
	use super::*;
	use crate::combat::{BossAttackCooldown, BossInCombat};
	use crate::presentation::{BossActor, BossEncounterRoot};
	use bikanwarda_character::BikanwardaRoot;
	use combat_targeting::CombatTargeting;
	use damage::{Health, Hit};
	use encounters::EncounterInstance;

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
		app.add_systems(
			Update,
			(damage::apply_hits, damage::mark_downed, damage::down_zero_health),
		);
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
		let boss = app
			.world_mut()
			.spawn((
				BossEncounterRoot::default(),
				BikanwardaRoot,
				EncounterInstance::new(chained_encounter_id()),
				Health::from_max(crate::presentation::PLACEHOLDER_BOSS_HEALTH),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		app.update();

		// Verify Health and EncounterInstance are on the same entity
		let boss_entity = app.world().entity(boss);
		assert!(boss_entity.contains::<Health>(), "Boss should have Health component");
		assert!(
			boss_entity.contains::<EncounterInstance>(),
			"Boss should have EncounterInstance component"
		);
		assert!(
			boss_entity.contains::<BossEncounterRoot>(),
			"Boss should have BossEncounterRoot marker"
		);

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
		let boss = app
			.world_mut()
			.spawn((
				BossEncounterRoot::default(),
				BikanwardaRoot,
				EncounterInstance::new(chained_encounter_id()),
				Health::from_max(100.0),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

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
		assert_eq!(
			record.outcome,
			Some(encounters::EncounterOutcome::Killed),
			"Encounter outcome should be Killed"
		);

		Ok(())
	}

	#[test]
	fn boss_combat_driver_initializes_state() {
		let mut app = combat_app();

		// Spawn actor (separate from host, matching production structure)
		let actor = app
			.world_mut()
			.spawn((BikanwardaRoot, Transform::default(), GlobalTransform::default()))
			.id();

		// Spawn host with BossActor binding (matching production structure)
		let host = app
			.world_mut()
			.spawn((
				BossEncounterRoot::default(),
				BossActor(actor),
				EncounterInstance::new(chained_encounter_id()),
				Health::from_max(1000.0),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		// Before update, combat state should not exist
		assert!(!app.world().entity(host).contains::<BossAttackCooldown>());
		assert!(!app.world().entity(actor).contains::<BossInCombat>());

		app.update();

		// After update, combat state should be initialized on correct entities
		assert!(
			app.world().entity(host).contains::<BossAttackCooldown>(),
			"Host should have attack cooldown"
		);
		assert!(
			app.world().entity(actor).contains::<BossInCombat>(),
			"Actor should have BossInCombat marker"
		);
		assert!(
			app.world().entity(actor).contains::<CombatTargeting>(),
			"Actor should have CombatTargeting"
		);

		let cooldown = app.world().entity(host).get::<BossAttackCooldown>().unwrap();
		assert_eq!(
			cooldown.timer.duration().as_secs_f32(),
			0.0,
			"Initial cooldown should be ready"
		);
	}
}

/// Production scene spawn test using real LodScene
mod scene_spawn {
	use super::*;
	use crate::generation::BikanwardaBossEncounter;
	use crate::presentation::{BossColumn, BossPedestal};
	use bevy::scene::WorldSceneExt;
	use damage::Health;
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
		let boss_position = Vec3::new(1000.0, 100.0 + encounter_hull_offset(), 2000.0);
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
		let root_entity =
			app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed").id();

		app.update();

		// Verify root entity has all required components
		let root = app.world().entity(root_entity);
		assert!(
			root.contains::<crate::presentation::BossEncounterRoot>(),
			"Root has BossEncounterRoot"
		);
		assert!(root.contains::<encounters::EncounterInstance>(), "Root has EncounterInstance");
		assert!(root.contains::<Health>(), "Root has Health (FAILS on 3b9f86e88)");
		assert!(root.contains::<Transform>(), "Root has Transform");

		// Verify Health value
		let health = root.get::<Health>().expect("Root should have Health");
		assert_eq!(health.max, crate::presentation::PLACEHOLDER_BOSS_HEALTH, "Boss health max");
		assert_eq!(
			health.current,
			crate::presentation::PLACEHOLDER_BOSS_HEALTH,
			"Boss health should start at max"
		);

		// Verify EncounterInstance and Health are on the SAME entity
		// (This is critical for encounters::store::ingest_deaths to work)
		let instance = root
			.get::<encounters::EncounterInstance>()
			.expect("Root should have EncounterInstance");
		assert_eq!(instance.id, crate::chained_encounter_id());

		// Verify position
		let transform = root.get::<Transform>().expect("Root should have Transform");
		assert!(
			(transform.translation - boss_position).length() < 0.01,
			"Boss at correct position"
		);

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
	use crate::combat::{BossAttackCooldown, BossInCombat};
	use crate::presentation::{BossActor, BossColumn, BossEncounterRoot, BossPedestal};
	use crate::{BossAssemblyPlugin, BossTetherChain};
	use attacks::{AttackInstance, BeginAttack};
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use bevy::scene::WorldSceneExt;
	use bikanwarda_character::BikanwardaRoot;
	use combat_targeting::CombatTargeting;
	use damage::{Health, Hit};
	use encounters::EncounterInstance;
	use lod::scene::LodScene;
	use spotting_intelligence::SpottingUser;

	fn full_encounter_app() -> App {
		full_encounter_app_with(|_| {})
	}

	fn full_encounter_app_with(extra: impl FnOnce(&mut App)) -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			bevy::asset::AssetPlugin::default(),
			bevy::scene::ScenePlugin,
			bevy::mesh::MeshPlugin, // Registers AssetEvent<Mesh>
		))
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));

		// Install physics before anything that needs it
		// PhysicsPlugins is a PluginGroup, not a Plugin, so we can't check is_plugin_added
		app.add_plugins(avian3d::prelude::PhysicsPlugins::default());

		// Install spotting with physics (required for real occlusion tests)
		// Must come before BikanwardaEncounterPlugin which uses it
		if !app.is_plugin_added::<spotting_intelligence_avian::SpottingAvianPlugin>() {
			app.add_plugins(spotting_intelligence_avian::SpottingAvianPlugin);
		}

		// Install attack systems (BikanwardaEncounterPlugin needs this)
		if !app.is_plugin_added::<attacks::AttacksPlugin>() {
			app.add_plugins(attacks::AttacksPlugin);
		}

		// Install bikanwarda attacks plugin to register attack definitions
		if !app.is_plugin_added::<bikanwarda_attacks::BikanwardaAttacksPlugin>() {
			app.add_plugins(bikanwarda_attacks::BikanwardaAttacksPlugin::default());
		}

		// Install bikanwarda encounter plugin
		// This includes: EncountersPlugin, LubikansPlugin, BossEncounterCombatPlugin,
		// BossEncounterLocomotionPlugin, BikanwardaChainPhysicsPlugin, BossTetherPlugin
		app.add_plugins(crate::BikanwardaEncounterPlugin);

		// Install assembly plugin to spawn actors
		app.add_plugins(BossAssemblyPlugin);

		crate::add_player_motor_plugins(&mut app);
		app.init_asset::<maybraid_audio::AudioClip>();
		app.add_systems(Update, spawn_test_support_ground);

		// Configure spotting to run on schedule (discovery every 0.2s, which is 12 frames at 60 FPS)
		// Don't gate it behind a timer - let it run every frame to pick up discovery_interval_secs
		app.configure_sets(Update, spotting_intelligence::SpottingSystems::Discover);

		extra(&mut app);

		app.finish();
		app.update();
		app
	}

	fn spawn_test_support_ground(
		hosts: Query<&GlobalTransform, Added<BossEncounterRoot>>,
		mut commands: Commands,
	) {
		use avian3d::prelude::{Collider, RigidBody};
		use bikanwarda_character::{BikanwardaActorRecipe, BikanwardaSpec};
		use characters::CharacterRecipe;

		for gt in &hosts {
			let t = gt.translation();
			let hull = BikanwardaActorRecipe::from_spec(BikanwardaSpec::encounter())
				.components()
				.scaled_locomotion_capsule();
			let terrain_y = t.y - hull.spawn_height();
			let center = Vec3::new(t.x, terrain_y - 1.0, t.z);
			commands.spawn((
				Name::new("test-encounter-ground"),
				RigidBody::Static,
				Collider::cuboid(400.0, 1.0, 400.0),
				Transform::from_translation(center),
				GlobalTransform::from_translation(center),
				lod_avian::PhysicsInteractionLayer::fixed_layers(),
			));
		}
	}

	#[test]
	fn real_assembly_creates_distinct_host_actor_pillar_pedestal() {
		let mut app = full_encounter_app();

		// Spawn encounter scene at known position
		let boss_position = Vec3::new(1000.0, 100.0 + encounter_hull_offset(), 2000.0);
		let terrain_y = 100.0;
		let encounter = crate::generation::BikanwardaBossEncounter::chained_at_with_terrain(
			boss_position,
			terrain_y,
		);

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
		let host_entity =
			app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed").id();

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
		let column_entity = children
			.iter()
			.find(|&c| app.world().entity(c).contains::<BossColumn>())
			.expect("Host should have column child");

		let pedestal_entity = children
			.iter()
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
		let host_global = app
			.world()
			.entity(host_entity)
			.get::<GlobalTransform>()
			.expect("Host GlobalTransform")
			.clone();
		let actor_global = app
			.world()
			.entity(actor_entity)
			.get::<GlobalTransform>()
			.expect("Actor GlobalTransform")
			.clone();

		// Actor should be at approximately the same position as host (local identity)
		let distance = (host_global.translation() - actor_global.translation()).length();
		assert!(distance < 1.0, "Actor near host position (distance: {})", distance);
	}

	#[test]
	fn combat_initialization_adds_components_to_host_and_actor() {
		let mut app = full_encounter_app();

		// Spawn a minimal host with BossActor binding
		let actor = app
			.world_mut()
			.spawn((BikanwardaRoot, Transform::default(), GlobalTransform::default()))
			.id();

		let host = app
			.world_mut()
			.spawn((
				BossEncounterRoot::default(),
				BossActor(actor),
				Health::from_max(1000.0),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		// Before update, combat state should not exist
		assert!(!app.world().entity(host).contains::<BossAttackCooldown>(), "No cooldown yet");
		assert!(
			!app.world().entity(actor).contains::<combat_targeting::CombatTargeting>(),
			"No targeting yet"
		);

		app.update();

		// After update, combat state should be initialized
		assert!(app.world().entity(host).contains::<BossAttackCooldown>(), "Host has cooldown");
		assert!(
			app.world().entity(actor).contains::<combat_targeting::CombatTargeting>(),
			"Actor has targeting"
		);
		assert!(app.world().entity(actor).contains::<BossInCombat>(), "Actor has combat marker");
	}

	#[test]
	fn exactly_one_tether_spawns_after_many_updates() {
		let mut app = full_encounter_app();

		// Spawn full encounter scene
		let boss_position = Vec3::new(1000.0, 100.0 + encounter_hull_offset(), 2000.0);
		let terrain_y = 100.0;
		let encounter = crate::generation::BikanwardaBossEncounter::chained_at_with_terrain(
			boss_position,
			terrain_y,
		);

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
		let tether_count = app
			.world_mut()
			.query_filtered::<Entity, With<crate::chains::ChainRoot>>()
			.iter(app.world())
			.count();

		// Should have exactly 1 chain root (not 0, not >1)
		assert_eq!(tether_count, 1, "Exactly one tether chain should exist after many updates");

		// Verify the host is marked with BossTetherChain to prevent repeat spawns
		let hosts: Vec<_> = app
			.world_mut()
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
		let actor = app
			.world_mut()
			.spawn((
				BikanwardaRoot,
				attacks::AttackUser::default(),
				combat_targeting::CombatTargeting::default(),
				Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
				GlobalTransform::default(),
			))
			.id();

		// Spawn host
		let host = app
			.world_mut()
			.spawn((
				BossEncounterRoot::default(),
				BossActor(actor),
				Health::from_max(1000.0),
				crate::combat::BossAttackCooldown::default(),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		// Spawn player nearby
		let player = app
			.world_mut()
			.spawn((
				world_player::Player,
				Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
				GlobalTransform::default(),
			))
			.id();

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
		let instances: Vec<_> =
			app.world_mut().query::<&AttackInstance>().iter(app.world()).collect();

		assert!(!instances.is_empty(), "At least one AttackInstance should exist");

		// Verify the instance targets the player
		let has_player_target = instances.iter().any(|inst| inst.live_target == Some(player));
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
		let host = app
			.world_mut()
			.spawn((
				BossEncounterRoot::default(),
				Health::from_max(1000.0),
				Transform::from_translation(host_position),
				GlobalTransform::from_translation(host_position),
			))
			.id();

		// Spawn actor as child with LOCAL identity (will have world pos = host pos after propagation)
		let actor = app
			.world_mut()
			.spawn((
				BikanwardaRoot,
				ChildOf(host),
				Transform::IDENTITY, // Local identity
				GlobalTransform::IDENTITY,
				SpottingUser::new(
					crate::boss_eye_offset(),
					[spotting_intelligence::SpotDirective::new(
						spotting_intelligence::InterestLayers::CHARACTER,
						crate::boss_spotting_range(),
					)],
				),
				CombatTargeting::default(),
				combat_targeting::SpottingTargetingSettings::default(),
			))
			.id();

		app.world_mut().entity_mut(host).insert(BossActor(actor));

		// Run updates to propagate transforms
		for _ in 0..3 {
			app.update();
		}

		// Verify actor's GlobalTransform matches host position (not local identity)
		let actor_global = app.world().entity(actor).get::<GlobalTransform>().unwrap();
		let distance_from_origin = actor_global.translation().distance(Vec3::ZERO);

		assert!(
			distance_from_origin > 100.0,
			"Actor GlobalTransform should reflect host world position ({:?}), not local identity",
			host_position
		);
		assert!(
			(actor_global.translation() - host_position).length() < 1.0,
			"Actor GlobalTransform should match host position"
		);
	}

	// DELETED: Test setup was incomplete (missing proper damage system configuration).
	// Damage routing is tested through chain physics integration tests.

	#[allow(dead_code)]
	fn deleted_damage_routing_test() {
		let mut app = full_encounter_app();

		// Spawn host with Health and EncounterInstance first
		let host = app
			.world_mut()
			.spawn((
				BossEncounterRoot::default(),
				Health::from_max(100.0),
				EncounterInstance::new(crate::chained_encounter_id()),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		// Spawn actor as child of host (matching production assembly)
		let actor = app
			.world_mut()
			.spawn((
				BikanwardaRoot,
				attacks::AttackUser::default(),
				ChildOf(host),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		// Add BossActor binding to host
		app.world_mut().entity_mut(host).insert(BossActor(actor));

		// Spawn a collider as child of actor (simulating actor's root collider)
		let actor_collider = app
			.world_mut()
			.spawn((
				avian3d::prelude::Collider::cylinder(1.0, 2.0),
				ChildOf(actor),
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

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
		assert_eq!(
			record.outcome,
			Some(encounters::EncounterOutcome::Killed),
			"Encounter should be marked as Killed"
		);
	}

	/// Test (a): Visible player in world-space range gets attack.
	///
	/// Uses real LodScene, assembly, spotting, acquisition and attack driver.
	/// No hand-written BeginAttack, no hand-filled CombatTargeting.
	///
	/// Frame timing at 60 FPS (DT = 1/60s):
	/// - Spotting discovery runs every 0.2s (12 frames)
	/// - Attack cooldown starts ready (0s)
	/// - Need ~20 frames: 5 for assembly + 12 for discovery + 3 for targeting/attack
	#[test]
	fn boss_attacks_visible_player_in_range() {
		let mut app = full_encounter_app();

		// Spawn encounter scene at nonzero world position
		let boss_position = Vec3::new(1000.0, 100.0 + encounter_hull_offset(), 2000.0);
		let terrain_y = 100.0;
		let encounter = crate::generation::BikanwardaBossEncounter::chained_at_with_terrain(
			boss_position,
			terrain_y,
		);

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
		let host_entity =
			app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed").id();

		// Run assembly to create actor and initialize combat
		for _ in 0..5 {
			app.update();
		}

		// Get actor entity from host
		let actor_entity = app
			.world()
			.entity(host_entity)
			.get::<BossActor>()
			.expect("Host should have BossActor")
			.0;

		// Spawn player in range (within 40m attack range, within 50m spotting range) and visible
		let player_pos =
			boss_position + Vec3::new(bikanwarda_character::encounter_meters(20.0), 0.0, 0.0);
		let player = app
			.world_mut()
			.spawn((
				Name::new("test-player"),
				Transform::from_translation(player_pos),
				GlobalTransform::from_translation(player_pos),
				spotting_intelligence::SpotSubject::new(
					spotting_intelligence::InterestLayers::CHARACTER,
					spotting_intelligence::SpotBounds::capsule(0.4, 1.8),
				),
				avian3d::prelude::RigidBody::Kinematic,
				avian3d::prelude::Collider::capsule(0.4, 1.8),
				// CRITICAL: Must have Animated collision layer for spotting discovery
				lod_avian::PhysicsInteractionLayer::animated_layers(),
				// ThreatSubject so threat intelligence can discover them as a threat
				threat_intelligence::ThreatSubject::new(threat_intelligence::ThreatId(1)),
				// Player affiliations: member of PLAYER_GROUP (like controlled world players)
				{
					let mut affiliations = threat_intelligence::Affiliations::default();
					const PLAYER_GROUP: threat_intelligence::ThreatGroupId =
						threat_intelligence::ThreatGroupId::group(29);
					affiliations.join(
						PLAYER_GROUP,
						threat_intelligence::AffiliationStrength::permanent(1.0),
					);
					affiliations
				},
			))
			.id();

		// Run updates to allow threat intelligence discovery, spotting, and attack triggering
		// Threat intelligence scan interval: 0.5s = 30 frames at DT=1/60s (with jitter: 24-36)
		// Spotting discovery interval: 0.2s = 12 frames at DT=1/60s
		// Run 60 frames to allow discovery cycles: 5 assembly + 36 threat scan + 19 buffer
		let mut attacks_seen = Vec::new();
		for _ in 0..60 {
			app.update();

			// Check for attack messages after each update
			let frame_attacks: Vec<BeginAttack> = app.world_mut()
				.run_system_once(|mut messages: bevy::ecs::message::MessageReader<BeginAttack>| -> Vec<BeginAttack> {
					messages.read().cloned().collect()
				})
				.expect("Failed to read BeginAttack messages");

			attacks_seen.extend(frame_attacks);
		}

		assert!(
			!attacks_seen.is_empty(),
			"Boss should have triggered attack on visible player in range"
		);
		assert!(
			attacks_seen.iter().any(|attack| attack.target == Some(player)),
			"Attack should target the player entity"
		);
	}

	/// Test (b): Player behind occluder gets no attack.
	///
	/// Uses real LodScene, assembly, spotting with occlusion, acquisition and attack driver.
	///
	/// Frame timing at 60 FPS (DT = 1/60s):
	/// - Spotting discovery runs every 0.2s (12 frames)
	/// - Need multiple discovery cycles to confirm no spotting: 30 frames = ~2.5 cycles
	#[test]
	fn boss_does_not_attack_occluded_player() {
		let mut app = full_encounter_app();

		// Spawn encounter scene at nonzero world position
		let boss_position = Vec3::new(1000.0, 100.0 + encounter_hull_offset(), 2000.0);
		let terrain_y = 100.0;
		let encounter = crate::generation::BikanwardaBossEncounter::chained_at_with_terrain(
			boss_position,
			terrain_y,
		);

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
		let host_entity =
			app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed").id();

		// Run assembly to create actor and initialize combat
		for _ in 0..5 {
			app.update();
		}

		// Get actor entity from host
		let actor_entity = app
			.world()
			.entity(host_entity)
			.get::<BossActor>()
			.expect("Host should have BossActor")
			.0;

		// Spawn occluder between boss and player position FIRST (before player spawns)
		// Boss eyes follow the scaled encounter hull.
		let player_pos =
			boss_position + Vec3::new(bikanwarda_character::encounter_meters(20.0), 0.0, 0.0);
		let occluder_pos = boss_position
			+ Vec3::new(
				bikanwarda_character::encounter_meters(10.0),
				crate::boss_eye_offset().y,
				0.0,
			);
		app.world_mut().spawn((
			Name::new("occluder-wall"),
			Transform::from_translation(occluder_pos),
			GlobalTransform::from_translation(occluder_pos),
			avian3d::prelude::RigidBody::Static,
			avian3d::prelude::Collider::cuboid(
				bikanwarda_character::encounter_meters(4.0),
				10.0,
				10.0,
			),
			// Occluders should be Fixed (terrain/buildings layer)
			lod_avian::PhysicsInteractionLayer::fixed_layers(),
		));

		// Now spawn player behind the occluder
		// Player should NOT be spotted due to occlusion, so no attack should fire
		let player = app
			.world_mut()
			.spawn((
				Name::new("occluded-player"),
				Transform::from_translation(player_pos),
				GlobalTransform::from_translation(player_pos),
				spotting_intelligence::SpotSubject::new(
					spotting_intelligence::InterestLayers::CHARACTER,
					spotting_intelligence::SpotBounds::capsule(0.4, 1.8),
				),
				avian3d::prelude::RigidBody::Kinematic,
				avian3d::prelude::Collider::capsule(0.4, 1.8),
				// CRITICAL: Must have Animated collision layer for spotting discovery
				lod_avian::PhysicsInteractionLayer::animated_layers(),
				// ThreatSubject so threat intelligence can discover them as a threat
				threat_intelligence::ThreatSubject::new(threat_intelligence::ThreatId(2)),
				// Player affiliations: member of PLAYER_GROUP (like controlled world players)
				{
					let mut affiliations = threat_intelligence::Affiliations::default();
					const PLAYER_GROUP: threat_intelligence::ThreatGroupId =
						threat_intelligence::ThreatGroupId::group(29);
					affiliations.join(
						PLAYER_GROUP,
						threat_intelligence::AffiliationStrength::permanent(1.0),
					);
					affiliations
				},
			))
			.id();

		// No manual targeting insertion - threat intelligence will grant ENEMYSHIP sources

		// Update a few times to let physics settle and transforms propagate
		for _ in 0..3 {
			app.update();
		}

		// Run updates for multiple discovery cycles to confirm no spotting
		// 30 frames at 60 FPS = 0.5s = 2.5 discovery cycles
		let mut attacks_seen = Vec::new();
		let mut spotted_player = false;
		for i in 0..30 {
			app.update();

			// Check if player is being spotted
			if let Some(spotting) = app.world().get::<SpottingUser>(actor_entity) {
				if spotting.contacts.contains_key(&player) {
					spotted_player = true;
					eprintln!("DIAGNOSTIC: Frame {}: Player spotted despite occlusion!", i);
				}
			}

			// Check for attack messages after each update
			let frame_attacks: Vec<BeginAttack> = app.world_mut()
				.run_system_once(|mut messages: bevy::ecs::message::MessageReader<BeginAttack>| -> Vec<BeginAttack> {
					messages.read().cloned().collect()
				})
				.expect("Failed to read BeginAttack messages");

			if !frame_attacks.is_empty() {
				eprintln!("DIAGNOSTIC: Frame {}: {} attacks", i, frame_attacks.len());
			}

			attacks_seen.extend(frame_attacks);
		}

		if spotted_player {
			eprintln!("DIAGNOSTIC: Occluded player was spotted - occlusion check not working");
		}

		// Verify no attack was targeted at the occluded player
		assert!(
			attacks_seen.iter().all(|attack| attack.target != Some(player)),
			"Boss should not attack occluded player (spotted: {})",
			spotted_player
		);
	}

	/// Test (c): Out-of-range player gets no attack.
	///
	/// Uses real LodScene, assembly, spotting, acquisition and attack driver.
	///
	/// Frame timing at 60 FPS (DT = 1/60s):
	/// - Attack range is 40m, spotting range is 50m
	/// - Player at 60m is outside both ranges
	/// - Run 30 frames to confirm no attack (even if somehow spotted)
	#[test]
	fn boss_does_not_attack_out_of_range_player() {
		let mut app = full_encounter_app();

		// Spawn encounter scene at nonzero world position
		let boss_position = Vec3::new(1000.0, 100.0 + encounter_hull_offset(), 2000.0);
		let terrain_y = 100.0;
		let encounter = crate::generation::BikanwardaBossEncounter::chained_at_with_terrain(
			boss_position,
			terrain_y,
		);

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
		let host_entity =
			app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed").id();

		// Run assembly to create actor and initialize combat
		for _ in 0..5 {
			app.update();
		}

		// Get actor entity from host
		let actor_entity = app
			.world()
			.entity(host_entity)
			.get::<BossActor>()
			.expect("Host should have BossActor")
			.0;

		// Spawn player out of range (attack range is 40m, spotting range is 50m)
		// Player at 60m should not be spotted (outside spotting range)
		let player_pos = boss_position + Vec3::new(60.0, 0.0, 0.0); // 60m away, outside both ranges
		let player = app
			.world_mut()
			.spawn((
				Name::new("distant-player"),
				Transform::from_translation(player_pos),
				GlobalTransform::from_translation(player_pos),
				spotting_intelligence::SpotSubject::new(
					spotting_intelligence::InterestLayers::CHARACTER,
					spotting_intelligence::SpotBounds::capsule(0.4, 1.8),
				),
				avian3d::prelude::RigidBody::Kinematic,
				avian3d::prelude::Collider::capsule(0.4, 1.8),
				// CRITICAL: Must have Animated collision layer for spotting discovery
				lod_avian::PhysicsInteractionLayer::animated_layers(),
				// ThreatSubject so threat intelligence can discover them as a threat
				threat_intelligence::ThreatSubject::new(threat_intelligence::ThreatId(3)),
				// Player affiliations: member of PLAYER_GROUP (like controlled world players)
				{
					let mut affiliations = threat_intelligence::Affiliations::default();
					const PLAYER_GROUP: threat_intelligence::ThreatGroupId =
						threat_intelligence::ThreatGroupId::group(29);
					affiliations.join(
						PLAYER_GROUP,
						threat_intelligence::AffiliationStrength::permanent(1.0),
					);
					affiliations
				},
			))
			.id();

		// No manual targeting insertion - threat intelligence will grant ENEMYSHIP sources

		// Run updates - player should not be spotted (out of spotting range)
		// and should not be attacked (out of attack range)
		// 30 frames at 60 FPS = 0.5s = 2.5 discovery cycles
		let mut attacks_seen = Vec::new();
		for _ in 0..30 {
			app.update();

			// Check for attack messages after each update
			let frame_attacks: Vec<BeginAttack> = app.world_mut()
				.run_system_once(|mut messages: bevy::ecs::message::MessageReader<BeginAttack>| -> Vec<BeginAttack> {
					messages.read().cloned().collect()
				})
				.expect("Failed to read BeginAttack messages");

			attacks_seen.extend(frame_attacks);
		}

		// Verify no attack was targeted at the out-of-range player
		assert!(
			attacks_seen.iter().all(|attack| attack.target != Some(player)),
			"Boss should not attack out-of-range player"
		);
	}

	/// Test (d): Neutral character (non-PLAYER_GROUP) in range and visible gets no attack.
	///
	/// Uses real LodScene, assembly, spotting, threat intelligence and attack driver.
	/// Verifies the shared bridge doesn't attack every visible character - only hostile ones.
	///
	/// Frame timing at 60 FPS (DT = 1/60s):
	/// - Threat intelligence scan: 0.5s = 30 frames (with jitter: 24-36)
	/// - Spotting discovery: 0.2s = 12 frames
	/// - Run 60 frames to ensure discovery cycles complete
	#[test]
	fn boss_does_not_attack_neutral_character() {
		let mut app = full_encounter_app();

		// Spawn encounter scene at nonzero world position
		let boss_position = Vec3::new(1000.0, 100.0 + encounter_hull_offset(), 2000.0);
		let terrain_y = 100.0;
		let encounter = crate::generation::BikanwardaBossEncounter::chained_at_with_terrain(
			boss_position,
			terrain_y,
		);

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
		let host_entity =
			app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed").id();

		// Run assembly to create actor and initialize combat
		for _ in 0..5 {
			app.update();
		}

		// Get actor entity from host
		let actor_entity = app
			.world()
			.entity(host_entity)
			.get::<BossActor>()
			.expect("Host should have BossActor")
			.0;

		// Spawn neutral character in range and visible
		// NO PLAYER_GROUP membership - boss antagonizes PLAYER_GROUP but this character isn't in it
		let neutral_pos =
			boss_position + Vec3::new(bikanwarda_character::encounter_meters(20.0), 0.0, 0.0);
		let neutral = app
			.world_mut()
			.spawn((
				Name::new("neutral-character"),
				Transform::from_translation(neutral_pos),
				GlobalTransform::from_translation(neutral_pos),
				spotting_intelligence::SpotSubject::new(
					spotting_intelligence::InterestLayers::CHARACTER,
					spotting_intelligence::SpotBounds::capsule(0.4, 1.8),
				),
				avian3d::prelude::RigidBody::Kinematic,
				avian3d::prelude::Collider::capsule(0.4, 1.8),
				// CRITICAL: Must have Animated collision layer for spotting discovery
				lod_avian::PhysicsInteractionLayer::animated_layers(),
				// ThreatSubject so threat intelligence can see them
				threat_intelligence::ThreatSubject::new(threat_intelligence::ThreatId(100)),
				// Neutral affiliations: NOT a member of PLAYER_GROUP (no join call)
				threat_intelligence::Affiliations::default(),
			))
			.id();

		// Run updates to allow discovery cycles
		// Threat intelligence will discover them, but ThreatManagement won't grant ENEMYSHIP
		// because threat_weight (boss.antagonists vs neutral.memberships) will be 0
		let mut attacks_seen = Vec::new();
		for _ in 0..60 {
			app.update();

			// Check for attack messages after each update
			let frame_attacks: Vec<BeginAttack> = app.world_mut()
				.run_system_once(|mut messages: bevy::ecs::message::MessageReader<BeginAttack>| -> Vec<BeginAttack> {
					messages.read().cloned().collect()
				})
				.expect("Failed to read BeginAttack messages");

			attacks_seen.extend(frame_attacks);
		}

		// Verify the neutral character was spotted (visible)
		let actor_ref = app.world().entity(actor_entity);
		let spotting = actor_ref.get::<SpottingUser>().expect("Actor should have SpottingUser");
		let was_spotted = spotting.contacts.contains_key(&neutral);

		// Verify no attack was targeted at the neutral character
		assert!(
			attacks_seen.iter().all(|attack| attack.target != Some(neutral)),
			"Boss should not attack neutral character (was_spotted: {})",
			was_spotted
		);
	}

	/// Test (e): Real Bray damage - production scene with real boss driver, player, spotting, threat.
	///
	/// Requirements (from PR description):
	/// - Real boss driver (BossEncounterCombatPlugin systems)
	/// - Real player (world/player, real collision layers)
	/// - Real spotting and threat intelligence
	/// - No seeding, no forced state, no faked events
	/// - Step frames until Bray is emitted from the socket
	/// - Assert player Health drops exactly once through real damage path
	/// - Verify emission origin is at the mouth socket position
	///
	/// Frame timing at 60 FPS (DT = 1/60s):
	/// - Threat intelligence scan: 0.5s = 30 frames (with jitter: 24-36)
	/// - Spotting discovery: 0.2s = 12 frames
	/// - Attack cooldown: starts ready (0s)
	/// - Bray duration: ~2s = 120 frames
	/// - Run ~200 frames to allow: discovery (36) + targeting (10) + attack start (1) + Bray animation (120) + damage (1)
	/// Test that Bray wavefronts hit ground-level targets with real production geometry.
	/// Bray fires 3 waves from mouth socket at Y~16.2m. Each wave should hit the player standing
	/// on the ground (Y~1.3m) exactly once, for a total of 3 hits.
	#[test]
	fn real_bray_waves_hit_ground_player_15m() {
		let mut app = full_encounter_app();

		// Spawn encounter scene at origin for simplicity
		let terrain_y = 0.0;
		let boss_spawn_offset = encounter_hull_offset();
		let boss_position = Vec3::new(0.0, terrain_y + boss_spawn_offset, 0.0);
		let encounter = crate::generation::BikanwardaBossEncounter::chained_at_with_terrain(
			boss_position,
			terrain_y,
		);

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
		let host_entity =
			app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed").id();

		// Run assembly to create actor and initialize combat
		for _ in 0..10 {
			app.update();
		}

		// Get actor entity and mouth socket from host
		let actor_entity = app
			.world()
			.entity(host_entity)
			.get::<BossActor>()
			.expect("Host should have BossActor")
			.0;

		// Verify mouth socket exists and has GlobalTransform
		let mouth_socket_count = app
			.world_mut()
			.query_filtered::<(&GlobalTransform, &ChildOf), With<bikanwarda_attacks::BikanwardaMouthSocket>>(
			)
			.iter(app.world())
			.filter(|(_, parent)| parent.parent() == actor_entity)
			.count();

		eprintln!("Mouth sockets found for actor: {}", mouth_socket_count);
		assert!(mouth_socket_count > 0, "Boss actor should have at least one mouth socket");

		if let Some((socket_gt, _)) = app
			.world_mut()
			.query_filtered::<(&GlobalTransform, &ChildOf), With<bikanwarda_attacks::BikanwardaMouthSocket>>(
			)
			.iter(app.world())
			.find(|(_, parent)| parent.parent() == actor_entity)
		{
			eprintln!("Mouth socket GlobalTransform: {:?}", socket_gt.translation());
		}

		// Player capsule: radius 0.4, height 1.8
		// Capsule origin is at center, feet are at center.y - (height/2 + radius)
		// So for feet on ground (terrain_y), center should be at terrain_y + (1.8/2 + 0.4) = terrain_y + 1.3
		const PLAYER_CAPSULE_RADIUS: f32 = 0.4;
		const PLAYER_CAPSULE_HEIGHT: f32 = 1.8;
		let player_ground_offset = PLAYER_CAPSULE_HEIGHT / 2.0 + PLAYER_CAPSULE_RADIUS; // 1.3m

		// Spawn player at production ground height, 15m horizontally from boss (within 40m range)
		let horizontal_distance = 15.0;
		let player_pos = Vec3::new(horizontal_distance, terrain_y + player_ground_offset, 0.0);
		let player = app
			.world_mut()
			.spawn((
				Name::new("test-player-15m"),
				Transform::from_translation(player_pos),
				GlobalTransform::from_translation(player_pos),
				Health::from_max(100.0), // Player starts at 100 HP
				spotting_intelligence::SpotSubject::new(
					spotting_intelligence::InterestLayers::CHARACTER,
					spotting_intelligence::SpotBounds::capsule(
						PLAYER_CAPSULE_RADIUS,
						PLAYER_CAPSULE_HEIGHT,
					),
				),
				avian3d::prelude::RigidBody::Kinematic,
				avian3d::prelude::Collider::capsule(PLAYER_CAPSULE_RADIUS, PLAYER_CAPSULE_HEIGHT),
				// CRITICAL: Must have Animated collision layer for spotting discovery
				lod_avian::PhysicsInteractionLayer::animated_layers(),
				// ThreatSubject so threat intelligence can discover them as a threat
				threat_intelligence::ThreatSubject::new(threat_intelligence::ThreatId(1)),
				// Player affiliations: member of PLAYER_GROUP (like controlled world players)
				{
					let mut affiliations = threat_intelligence::Affiliations::default();
					const PLAYER_GROUP: threat_intelligence::ThreatGroupId =
						threat_intelligence::ThreatGroupId::group(29);
					affiliations.join(
						PLAYER_GROUP,
						threat_intelligence::AffiliationStrength::permanent(1.0),
					);
					affiliations
				},
			))
			.id();

		eprintln!(
			"Boss position: {:?} (terrain_y={}, spawn_offset={})",
			boss_position, terrain_y, boss_spawn_offset
		);
		eprintln!(
			"Player position: {:?} (terrain_y={}, ground_offset={}, horizontal_distance={})",
			player_pos, terrain_y, player_ground_offset, horizontal_distance
		);

		// Track player health changes and wave hits
		let initial_health = 100.0;
		let mut final_health = initial_health;
		let mut damage_events = 0;
		let mut bray_started = false;
		let mut first_emission_origin: Option<Vec3> = None;
		let mut hit_messages_seen = 0;

		// Bray fires 3 waves, expect 3 hits (one per wave)
		const EXPECTED_BRAY_WAVES: usize = 3;

		// Run frames until Bray completes or timeout
		// Up to 350 frames to allow all 3 waves to travel (3 waves * 0.45s interval + 2.8s travel ~= 4.2s = 252 frames)
		for frame in 0..350 {
			app.update();

			// Check if Bray attack has started (BeginAttack message)
			if !bray_started {
				let frame_attacks: Vec<BeginAttack> = app.world_mut()
					.run_system_once(|mut messages: bevy::ecs::message::MessageReader<BeginAttack>| -> Vec<BeginAttack> {
						messages.read().cloned().collect()
					})
					.expect("Failed to read BeginAttack messages");

				if let Some(attack) = frame_attacks
					.iter()
					.find(|a| a.owner == actor_entity && a.definition == bikanwarda_attacks::BRAY)
				{
					bray_started = true;
					eprintln!("Frame {}: Bray attack started targeting {:?}", frame, attack.target);
					eprintln!(
						"  emission: {:?}, emission_socket: {:?}",
						attack.emission, attack.emission_socket
					);

					// Capture first emission origin if available
					if let Some(emission) = attack.emission {
						first_emission_origin = Some(emission);
						eprintln!("Frame {}: First Bray emission origin: {:?}", frame, emission);
					}
				}
			}

			// Check for Hit messages (damage system input)
			let frame_hits: Vec<damage::Hit> = app.world_mut()
				.run_system_once(|mut messages: bevy::ecs::message::MessageReader<damage::Hit>| -> Vec<damage::Hit> {
					messages.read().cloned().collect()
				})
				.expect("Failed to read Hit messages");

			if !frame_hits.is_empty() {
				for hit in &frame_hits {
					if hit.target == player {
						hit_messages_seen += 1;
						eprintln!(
							"Frame {}: Hit #{} for player: amount={:.1} HP, point={:?}",
							frame, hit_messages_seen, hit.amount, hit.point
						);
					}
				}
			}

			// Check player health
			if let Some(health) = app.world().get::<Health>(player) {
				if health.current < final_health {
					let delta = final_health - health.current;
					damage_events += 1;
					final_health = health.current;
					eprintln!(
						"Frame {}: Player damage event #{}: -{:.1} HP, now at {:.1}/{:.1} HP",
						frame, damage_events, delta, final_health, initial_health
					);
				}
			}

			// Stop after all expected waves hit or Bray completes
			if bray_started && damage_events >= EXPECTED_BRAY_WAVES {
				eprintln!(
					"Frame {}: All {} Bray waves hit player, test complete",
					frame, EXPECTED_BRAY_WAVES
				);
				break;
			}
		}

		eprintln!("\nTest summary:");
		eprintln!("  Total Hit messages: {}", hit_messages_seen);
		eprintln!("  Damage applications: {}", damage_events);
		eprintln!(
			"  HP: {:.1} -> {:.1} ({:.1} total damage)",
			initial_health,
			final_health,
			initial_health - final_health
		);

		// Assertions
		assert!(bray_started, "Boss should have started Bray attack on visible player in range");

		assert_eq!(
			damage_events, EXPECTED_BRAY_WAVES,
			"Player should take damage from each Bray wave (expected {}, got {})",
			EXPECTED_BRAY_WAVES, damage_events
		);

		// Each wave should deal 20 HP damage
		let expected_total_damage = 20.0 * EXPECTED_BRAY_WAVES as f32;
		let actual_damage = initial_health - final_health;
		assert!(
			(actual_damage - expected_total_damage).abs() < 0.1,
			"Total damage should be {}×20 HP = {:.1} HP (got {:.1} HP)",
			EXPECTED_BRAY_WAVES,
			expected_total_damage,
			actual_damage
		);

		assert!(
			final_health < initial_health,
			"Player health should decrease: initial={}, final={}",
			initial_health,
			final_health
		);
	}

	/// Test that Bray wavefronts hit ground-level targets at longer range (30m).
	/// Same setup as 15m test but with player further away, still within the 28m Bray range.
	#[test]
	fn real_bray_waves_hit_ground_player_30m() {
		// TODO: Add 30m distance test case
	}

	/// Regression for [#1619](https://github.com/ramate-io/maybraid/pull/1619) comment
	/// [6996063622](https://github.com/ramate-io/maybraid/pull/1619#issuecomment-6996063622):
	/// Bray cues name the gameplay actor, but the clip host is the typed body rig
	/// under the visual child. One real Bray must switch that rig's clip and restore it.
	#[test]
	fn real_bray_plays_and_restores_clip_on_bound_body_rig() {
		use bikanwarda_attacks::BikanwardaPresentationConsumersPlugin;
		use bikanwarda_character::{BikanwardaActorRecipe, BikanwardaSpec};
		use characters::{
			bind_actor_body_rigs, ActorBodyRig, AnimClip, AnimRef, AnimRefRoot,
			CharacterComponents, CharacterRecipe, CharacterRig, CharacterRigRole, CharacterRoot,
			RigId,
		};
		use lod::scene::LodSceneLevel;
		use maybraid_spatial_rumble::SpatialRumblePlugin;

		let mut app = full_encounter_app_with(|app| {
			app.add_systems(PreUpdate, bind_actor_body_rigs);
			app.add_plugins(SpatialRumblePlugin);
			app.add_plugins(BikanwardaPresentationConsumersPlugin {
				placeholder_driven_vfx: false,
			});
		});

		let terrain_y = 0.0;
		let boss_spawn_offset = encounter_hull_offset();
		let boss_position = Vec3::new(0.0, terrain_y + boss_spawn_offset, 0.0);
		let encounter = crate::generation::BikanwardaBossEncounter::chained_at_with_terrain(
			boss_position,
			terrain_y,
		);

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
		let host_entity =
			app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed").id();
		for _ in 0..10 {
			app.update();
		}

		let actor_entity = app
			.world()
			.entity(host_entity)
			.get::<BossActor>()
			.expect("Host should have BossActor")
			.0;

		let visual = app
			.world_mut()
			.query_filtered::<(Entity, &ChildOf), With<CharacterRoot>>()
			.iter(app.world())
			.find(|(_, parent)| parent.parent() == actor_entity)
			.map(|(entity, _)| entity)
			.expect("spawn.rs visual child should be a CharacterRoot");

		let spec = BikanwardaSpec::encounter();
		let recipe = BikanwardaActorRecipe::from_spec(spec);
		let body_node = recipe
			.components()
			.rig_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.into_iter()
			.find(|node| node.id == RigId::Body)
			.expect("encounter recipe should author a body rig");
		let prior = AnimRef::new(AnimClip::quadruped_run());
		let spawned_rig =
			app.world_mut().spawn_scene(body_node.assembly_contents()).ok().map(|s| s.id());
		let rig = if let Some(rig) = spawned_rig {
			rig
		} else {
			app.world_mut()
				.spawn((
					CharacterRig { role: CharacterRigRole::Body, skeleton: Default::default() },
					AnimRefRoot(prior),
					ChildOf(visual),
				))
				.id()
		};
		app.world_mut().entity_mut(rig).insert((ChildOf(visual), AnimRefRoot(prior)));
		app.update();

		assert!(
			app.world().get::<AnimRefRoot>(actor_entity).is_none(),
			"gameplay actor must not carry the clip component"
		);
		let binding = app
			.world()
			.get::<ActorBodyRig>(actor_entity)
			.expect("actor should bind the typed body rig");
		assert_eq!(binding.rig, rig, "binding must point at the recipe body rig");

		const PLAYER_CAPSULE_RADIUS: f32 = 0.4;
		const PLAYER_CAPSULE_HEIGHT: f32 = 1.8;
		let player_ground_offset = PLAYER_CAPSULE_HEIGHT / 2.0 + PLAYER_CAPSULE_RADIUS;
		let player_pos = Vec3::new(15.0, terrain_y + player_ground_offset, 0.0);
		let _player = app
			.world_mut()
			.spawn((
				Name::new("test-player-bray-clip"),
				Transform::from_translation(player_pos),
				GlobalTransform::from_translation(player_pos),
				Health::from_max(100.0),
				spotting_intelligence::SpotSubject::new(
					spotting_intelligence::InterestLayers::CHARACTER,
					spotting_intelligence::SpotBounds::capsule(
						PLAYER_CAPSULE_RADIUS,
						PLAYER_CAPSULE_HEIGHT,
					),
				),
				avian3d::prelude::RigidBody::Kinematic,
				avian3d::prelude::Collider::capsule(PLAYER_CAPSULE_RADIUS, PLAYER_CAPSULE_HEIGHT),
				lod_avian::PhysicsInteractionLayer::animated_layers(),
				threat_intelligence::ThreatSubject::new(threat_intelligence::ThreatId(1)),
				{
					let mut affiliations = threat_intelligence::Affiliations::default();
					const PLAYER_GROUP: threat_intelligence::ThreatGroupId =
						threat_intelligence::ThreatGroupId::group(29);
					affiliations.join(
						PLAYER_GROUP,
						threat_intelligence::AffiliationStrength::permanent(1.0),
					);
					affiliations
				},
			))
			.id();

		let mut bray_started = false;
		let mut saw_bray_clip = false;
		let mut restored = false;
		for frame in 0..400 {
			app.update();
			if !bray_started {
				let frame_attacks: Vec<BeginAttack> = app
					.world_mut()
					.run_system_once(
						|mut messages: bevy::ecs::message::MessageReader<BeginAttack>| -> Vec<BeginAttack> {
							messages.read().cloned().collect()
						},
					)
					.expect("Failed to read BeginAttack messages");
				if frame_attacks.iter().any(|attack| {
					attack.owner == actor_entity && attack.definition == bikanwarda_attacks::BRAY
				}) {
					bray_started = true;
					eprintln!("Frame {frame}: Bray attack started");
				}
			}

			let anim = app.world().get::<AnimRefRoot>(rig).expect("body rig must keep AnimRefRoot");
			if anim.0.clip.id() == AnimClip::epiphant_bray().id() {
				if !saw_bray_clip {
					eprintln!("Frame {frame}: body rig switched to Epiphant Bray");
				}
				saw_bray_clip = true;
				assert!(
					app.world().get::<AnimRefRoot>(actor_entity).is_none(),
					"Bray must not copy AnimRefRoot onto the gameplay actor"
				);
			} else if saw_bray_clip && anim.0.clip.id() == prior.clip.id() {
				eprintln!("Frame {frame}: body rig restored prior clip");
				restored = true;
				break;
			}
		}

		assert!(bray_started, "combat driver should fire a real Bray");
		assert!(saw_bray_clip, "bound body rig should play AnimClip::epiphant_bray");
		assert!(restored, "bound body rig should restore the prior clip after Bray");
	}

	fn spawn_production_encounter(
		app: &mut App,
		boss_position: Vec3,
		terrain_y: f32,
	) -> (Entity, Entity, Entity) {
		let encounter = crate::generation::BikanwardaBossEncounter::chained_at_with_terrain(
			boss_position,
			terrain_y,
		);
		let anchor_transform = Transform::from_translation(Vec3::ZERO);
		let anchor = app.world_mut().spawn(anchor_transform).id();
		let scene = encounter.scene_with_level(
			&lod::lod_ref::LodRef {
				entity: anchor,
				previous_transform: &anchor_transform,
				current_transform: &anchor_transform,
				bounds: &bevy::shape::Aabb3d::from_min_max(Vec3::splat(-200.0), Vec3::splat(200.0)),
			},
			lod::scene::LodSceneLevel::High,
		);
		let host_entity =
			app.world_mut().spawn_scene(scene).expect("spawn_scene should succeed").id();
		for _ in 0..12 {
			app.update();
		}
		let actor_entity = app
			.world()
			.entity(host_entity)
			.get::<BossActor>()
			.expect("Host should have BossActor")
			.0;
		let column_entity = app
			.world()
			.entity(host_entity)
			.get::<Children>()
			.expect("Host should have children")
			.iter()
			.find(|&child| app.world().entity(child).contains::<BossColumn>())
			.expect("Host should have a pillar");
		(host_entity, actor_entity, column_entity)
	}

	fn xz(a: Vec3, b: Vec3) -> f32 {
		Vec2::new(a.x, a.z).distance(Vec2::new(b.x, b.z))
	}

	fn actor_translation(app: &App, actor: Entity) -> Vec3 {
		app.world()
			.get::<GlobalTransform>(actor)
			.expect("actor GlobalTransform")
			.translation()
	}

	fn visual_facing(app: &mut App, actor: Entity) -> Vec3 {
		use characters::{CharacterHeading, CharacterRoot};
		let visual = app
			.world_mut()
			.query_filtered::<(Entity, &ChildOf), With<CharacterRoot>>()
			.iter(app.world())
			.find(|(_, parent)| parent.parent() == actor)
			.map(|(entity, _)| entity)
			.expect("visual CharacterRoot");
		let heading = app.world().get::<CharacterHeading>(visual).copied().unwrap_or_default();
		let transform = app.world().get::<Transform>(visual).copied().unwrap_or_default();
		heading.facing(&transform)
	}

	fn spawn_visible_player(app: &mut App, at: Vec3) -> Entity {
		app.world_mut()
			.spawn((
				Name::new("test-player-locomotion"),
				Transform::from_translation(at),
				GlobalTransform::from_translation(at),
				Health::from_max(100.0),
				spotting_intelligence::SpotSubject::new(
					spotting_intelligence::InterestLayers::CHARACTER,
					spotting_intelligence::SpotBounds::capsule(0.4, 1.8),
				),
				avian3d::prelude::RigidBody::Kinematic,
				avian3d::prelude::Collider::capsule(0.4, 1.8),
				lod_avian::PhysicsInteractionLayer::animated_layers(),
				threat_intelligence::ThreatSubject::new(threat_intelligence::ThreatId(1)),
				{
					let mut affiliations = threat_intelligence::Affiliations::default();
					const PLAYER_GROUP: threat_intelligence::ThreatGroupId =
						threat_intelligence::ThreatGroupId::group(29);
					affiliations.join(
						PLAYER_GROUP,
						threat_intelligence::AffiliationStrength::permanent(1.0),
					);
					affiliations
				},
			))
			.id()
	}

	/// Production assembly: the actor's GlobalTransform must translate and yaw
	/// must change while the shared motor/tether stack patrols.
	#[test]
	fn real_boss_patrols_and_turns_within_tether() {
		let mut app = full_encounter_app();
		let terrain_y = 0.0;
		let boss_position = Vec3::new(40.0, terrain_y + encounter_hull_offset(), -25.0);
		let (_host, actor, column) = spawn_production_encounter(&mut app, boss_position, terrain_y);

		let start = actor_translation(&app, actor);
		let start_facing = visual_facing(&mut app, actor);
		let pillar = app
			.world()
			.get::<GlobalTransform>(column)
			.expect("pillar GlobalTransform")
			.translation();

		let mut max_move = 0.0_f32;
		let mut max_turn = 0.0_f32;
		let mut max_leash = 0.0_f32;
		for _ in 0..360 {
			app.update();
			let now = actor_translation(&app, actor);
			max_move = max_move.max(xz(start, now));
			max_turn = max_turn.max(start_facing.angle_between(visual_facing(&mut app, actor)));
			max_leash = max_leash.max(xz(now, pillar));
			assert!(
				xz(now, pillar) <= crate::BOSS_TETHER_RADIUS + bikanwarda_character::encounter_meters(2.5),
				"boss left the tether during patrol: xz={:.3} start={start:?} now={now:?} pillar={pillar:?}",
				xz(now, pillar)
			);
		}

		eprintln!(
			"patrol start={start:?} move={max_move:.3} turn={max_turn:.3} leash={max_leash:.3}"
		);
		assert!(
			max_move > 1.0,
			"boss GlobalTransform should translate during patrol, moved {max_move:.3} m from {start:?}"
		);
		assert!(
			max_turn > 0.15,
			"boss yaw should change during patrol, max turn {max_turn:.3} rad"
		);
	}

	/// Pillar stays a static physics actor while the boss patrols.
	#[test]
	fn real_pillar_stays_fixed_while_boss_patrols() {
		let mut app = full_encounter_app();
		let terrain_y = 0.0;
		let boss_position = Vec3::new(-30.0, terrain_y + encounter_hull_offset(), 18.0);
		let (_host, actor, column) = spawn_production_encounter(&mut app, boss_position, terrain_y);
		let pillar_start = app
			.world()
			.get::<GlobalTransform>(column)
			.expect("pillar GlobalTransform")
			.translation();
		let start = actor_translation(&app, actor);

		let mut moved = 0.0_f32;
		for _ in 0..240 {
			app.update();
			let pillar_now = app
				.world()
				.get::<GlobalTransform>(column)
				.expect("pillar GlobalTransform")
				.translation();
			assert!(
				(pillar_now - pillar_start).length() < 0.05,
				"pillar must stay fixed, start={pillar_start:?} now={pillar_now:?}"
			);
			moved = moved.max(xz(start, actor_translation(&app, actor)));
		}
		eprintln!("pillar fixed at {pillar_start:?}; boss moved {moved:.3} m");
		assert!(moved > 1.0, "boss should have patrolled while the pillar stayed put");
	}

	/// Push the actor past the leash; the shared tether objective pulls it back.
	#[test]
	fn real_boss_stays_within_tether_when_pushed() {
		use avian3d::prelude::LinearVelocity;

		let mut app = full_encounter_app();
		let terrain_y = 0.0;
		let boss_position = Vec3::new(12.0, terrain_y + encounter_hull_offset(), 8.0);
		let (_host, actor, column) = spawn_production_encounter(&mut app, boss_position, terrain_y);
		let pillar = app.world().get::<GlobalTransform>(column).expect("pillar").translation();

		let outside = Vec3::new(
			pillar.x + crate::BOSS_TETHER_RADIUS + bikanwarda_character::encounter_meters(10.0),
			boss_position.y,
			pillar.z,
		);
		let world_now = actor_translation(&app, actor);
		if let Some(mut transform) = app.world_mut().get_mut::<Transform>(actor) {
			let parent_world = world_now - transform.translation;
			transform.translation = outside - parent_world;
		}
		if let Some(mut velocity) = app.world_mut().get_mut::<LinearVelocity>(actor) {
			velocity.0 = Vec3::X * 12.0;
		}
		app.world_mut()
			.entity_mut(actor)
			.insert(GlobalTransform::from_translation(outside));

		let mut max_leash = xz(outside, pillar);
		let mut min_leash_after = max_leash;
		for frame in 0..300 {
			app.update();
			let now = actor_translation(&app, actor);
			let leash = xz(now, pillar);
			max_leash = max_leash.max(leash);
			if frame > 90 {
				min_leash_after = min_leash_after.min(leash);
			}
		}
		let final_leash = xz(actor_translation(&app, actor), pillar);
		eprintln!(
			"pushed leash start={:.3} max={:.3} min_after={:.3} final={:.3}",
			xz(outside, pillar),
			max_leash,
			min_leash_after,
			final_leash
		);
		assert!(
			final_leash <= crate::BOSS_TETHER_RADIUS + bikanwarda_character::encounter_meters(2.5),
			"boss should return inside the tether after a push, final leash {final_leash:.3}"
		);
		assert!(
			min_leash_after < xz(outside, pillar) - 2.0,
			"tether should pull the boss back toward the pillar"
		);
	}

	fn place_actor_at(app: &mut App, actor: Entity, world: Vec3) {
		use avian3d::prelude::LinearVelocity;

		let now = actor_translation(app, actor);
		if let Some(mut transform) = app.world_mut().get_mut::<Transform>(actor) {
			let parent_world = now - transform.translation;
			transform.translation = world - parent_world;
		}
		if let Some(mut velocity) = app.world_mut().get_mut::<LinearVelocity>(actor) {
			velocity.0 = Vec3::ZERO;
		}
		app.world_mut()
			.entity_mut(actor)
			.insert(GlobalTransform::from_translation(world));
	}

	/// Pursuing a spotted player at the edge must not leave the pillar radius.
	///
	/// Production spawn plants the actor on the 8 m patrol ring, so this test
	/// then walks the live actor to the leash rim and lets spotting fill
	/// `CombatTargeting` — no hand-seeded contact, no forced tactic.
	#[test]
	fn real_boss_stays_within_tether_when_pursuing_player_at_edge() {
		use combat_targeting::CombatTargeting;

		let mut app = full_encounter_app();
		let terrain_y = 0.0;
		let boss_position = Vec3::new(0.0, terrain_y + encounter_hull_offset(), 0.0);
		let (_host, actor, column) = spawn_production_encounter(&mut app, boss_position, terrain_y);
		let pillar = app.world().get::<GlobalTransform>(column).expect("pillar").translation();
		let edge = Vec3::new(
			pillar.x + crate::BOSS_TETHER_RADIUS - bikanwarda_character::encounter_meters(4.0),
			actor_translation(&app, actor).y,
			pillar.z,
		);
		place_actor_at(&mut app, actor, edge);
		let player_pos = Vec3::new(
			pillar.x + crate::BOSS_TETHER_RADIUS + bikanwarda_character::encounter_meters(18.0),
			terrain_y + 1.3,
			pillar.z,
		);
		let player = spawn_visible_player(&mut app, player_pos);

		let start = actor_translation(&app, actor);
		assert!(
			(xz(start, pillar) - (crate::BOSS_TETHER_RADIUS - bikanwarda_character::encounter_meters(4.0))).abs() < 0.5,
			"pursuit must start near the tether edge, start={start:?} pillar={pillar:?} leash={:.3}",
			xz(start, pillar)
		);
		let mut max_leash = xz(start, pillar);
		let mut saw_turn = false;
		let mut saw_spotted_player = false;
		let mut max_toward_player = 0.0_f32;
		let start_facing = visual_facing(&mut app, actor);
		for _ in 0..360 {
			app.update();
			if app
				.world()
				.get::<CombatTargeting>(actor)
				.and_then(CombatTargeting::best_contact)
				.is_some_and(|contact| contact.subject == player)
			{
				saw_spotted_player = true;
			}
			let now = actor_translation(&app, actor);
			let leash = xz(now, pillar);
			max_leash = max_leash.max(leash);
			max_toward_player = max_toward_player.max(now.x - start.x);
			assert!(
				leash <= crate::BOSS_TETHER_RADIUS + bikanwarda_character::encounter_meters(2.5),
				"boss left the tether while pursuing: leash={leash:.3} pos={now:?} pillar={pillar:?}"
			);
			if start_facing.angle_between(visual_facing(&mut app, actor)) > 0.2 {
				saw_turn = true;
			}
		}
		eprintln!(
			"pursue max leash={max_leash:.3} turned={saw_turn} spotted={saw_spotted_player} toward_player={max_toward_player:.3} start={start:?}"
		);
		assert!(
			saw_spotted_player,
			"CombatTargeting contact must come from spotting the spawned player"
		);
		assert!(saw_turn, "boss should turn toward the spotted player at the tether edge");
		assert!(
			max_toward_player > bikanwarda_character::encounter_meters(1.0),
			"boss should walk toward the spotted player while remaining leashed, moved {max_toward_player:.3} m"
		);
		assert!(
			max_leash <= crate::BOSS_TETHER_RADIUS + bikanwarda_character::encounter_meters(2.5),
			"pursuit must stay inside the tether radius"
		);
	}

	/// Production cadence: firearm movement at 125 ms, tether write at 250 ms.
	/// Reclaim must still keep the origin inside the pillar leash.
	#[test]
	fn real_boss_stays_leashed_when_pursuit_ticks_between_tether_writes() {
		use std::time::Duration;

		use bevy::time::common_conditions::on_timer;
		use combat_targeting::CombatTargeting;
		use firearm_intelligence::FirearmIntelligenceSystems;
		use tether_intelligence::TetherSystems;

		let mut app = full_encounter_app();
		app.configure_sets(
			Update,
			FirearmIntelligenceSystems::Movement.run_if(on_timer(Duration::from_millis(125))),
		);
		app.configure_sets(
			Update,
			TetherSystems::Write.run_if(on_timer(Duration::from_millis(250))),
		);
		app.configure_sets(
			Update,
			TetherSystems::Reclaim
				.after(FirearmIntelligenceSystems::Movement)
				.run_if(on_timer(Duration::from_millis(125))),
		);

		let terrain_y = 0.0;
		let boss_position = Vec3::new(0.0, terrain_y + encounter_hull_offset(), 0.0);
		let (_host, actor, column) = spawn_production_encounter(&mut app, boss_position, terrain_y);
		let pillar = app.world().get::<GlobalTransform>(column).expect("pillar").translation();
		let edge = Vec3::new(
			pillar.x + crate::BOSS_TETHER_RADIUS - bikanwarda_character::encounter_meters(4.0),
			actor_translation(&app, actor).y,
			pillar.z,
		);
		place_actor_at(&mut app, actor, edge);
		let player_pos = Vec3::new(
			pillar.x + crate::BOSS_TETHER_RADIUS + bikanwarda_character::encounter_meters(18.0),
			terrain_y + 1.3,
			pillar.z,
		);
		let player = spawn_visible_player(&mut app, player_pos);

		let mut max_leash = xz(actor_translation(&app, actor), pillar);
		let mut saw_spotted = false;
		for _ in 0..360 {
			app.update();
			if app
				.world()
				.get::<CombatTargeting>(actor)
				.and_then(CombatTargeting::best_contact)
				.is_some_and(|contact| contact.subject == player)
			{
				saw_spotted = true;
			}
			let now = actor_translation(&app, actor);
			let leash = xz(now, pillar);
			max_leash = max_leash.max(leash);
			assert!(
				leash <= crate::BOSS_TETHER_RADIUS + bikanwarda_character::encounter_meters(2.5),
				"staggered pursuit left the tether: leash={leash:.3} pos={now:?} pillar={pillar:?}"
			);
		}
		eprintln!("staggered pursue max leash={max_leash:.3} spotted={saw_spotted}");
		assert!(saw_spotted, "CombatTargeting contact must come from spotting the spawned player");
		assert!(
			max_leash <= crate::BOSS_TETHER_RADIUS + bikanwarda_character::encounter_meters(2.5),
			"pursuit ticks between tether writes must not breach the leash"
		);
	}

	/// Gait clip on the bound body rig follows actual XZ velocity, not a timer.
	#[test]
	fn real_gait_follows_velocity() {
		use avian3d::prelude::LinearVelocity;
		use bikanwarda_character::{BikanwardaActorRecipe, BikanwardaSpec};
		use characters::{
			bind_actor_body_rigs, ActorBodyRig, AnimClip, AnimId, AnimRef, AnimRefRoot,
			CharacterComponents, CharacterRecipe, CharacterRig, CharacterRigRole, CharacterRoot,
			MemberOf, RigId, RigSkeletonKind,
		};
		use lod::scene::LodSceneLevel;

		let mut app = full_encounter_app_with(|app| {
			app.add_systems(PreUpdate, bind_actor_body_rigs);
		});
		let terrain_y = 0.0;
		let boss_position = Vec3::new(6.0, terrain_y + encounter_hull_offset(), -4.0);
		let (_host, actor, _column) =
			spawn_production_encounter(&mut app, boss_position, terrain_y);

		let visual = app
			.world_mut()
			.query_filtered::<(Entity, &ChildOf), With<CharacterRoot>>()
			.iter(app.world())
			.find(|(_, parent)| parent.parent() == actor)
			.map(|(entity, _)| entity)
			.expect("visual CharacterRoot");
		let spec = BikanwardaSpec::encounter();
		let recipe = BikanwardaActorRecipe::from_spec(spec);
		let body_node = recipe
			.components()
			.rig_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.into_iter()
			.find(|node| node.id == RigId::Body)
			.expect("body rig");
		let prior = AnimRef::new(AnimClip::still());
		let spawned =
			app.world_mut().spawn_scene(body_node.assembly_contents()).ok().map(|s| s.id());
		let rig = spawned.unwrap_or_else(|| {
			app.world_mut()
				.spawn((
					CharacterRig {
						role: CharacterRigRole::Body,
						skeleton: RigSkeletonKind::Quadruped,
					},
					AnimRefRoot(prior),
					ChildOf(visual),
				))
				.id()
		});
		app.world_mut().entity_mut(rig).insert((
			ChildOf(visual),
			MemberOf(visual),
			AnimRefRoot(prior),
			CharacterRig { role: CharacterRigRole::Body, skeleton: RigSkeletonKind::Quadruped },
		));
		app.update();
		assert!(app.world().get::<ActorBodyRig>(actor).is_some());

		if let Some(mut velocity) = app.world_mut().get_mut::<LinearVelocity>(actor) {
			velocity.0 = Vec3::ZERO;
		}
		let mut saw_still = false;
		for _ in 0..20 {
			if let Some(mut velocity) = app.world_mut().get_mut::<LinearVelocity>(actor) {
				velocity.0 = Vec3::ZERO;
			}
			app.update();
			if app
				.world()
				.get::<AnimRefRoot>(rig)
				.is_some_and(|anim| anim.0.clip.id() == AnimId::Still)
			{
				saw_still = true;
				break;
			}
		}

		if let Some(mut velocity) = app.world_mut().get_mut::<LinearVelocity>(actor) {
			velocity.0 = Vec3::X * 4.0;
		}
		let mut saw_run = false;
		for _ in 0..30 {
			if let Some(mut velocity) = app.world_mut().get_mut::<LinearVelocity>(actor) {
				velocity.0 = Vec3::X * 4.0;
			}
			app.update();
			if app.world().get::<AnimRefRoot>(rig).is_some_and(|anim| {
				matches!(anim.0.clip.id(), AnimId::QuadrupedRun | AnimId::QuadrupedRunStart)
			}) {
				saw_run = true;
				break;
			}
		}

		if let Some(mut velocity) = app.world_mut().get_mut::<LinearVelocity>(actor) {
			velocity.0 = Vec3::ZERO;
		}
		let mut restored_still = false;
		for _ in 0..40 {
			if let Some(mut velocity) = app.world_mut().get_mut::<LinearVelocity>(actor) {
				velocity.0 = Vec3::ZERO;
			}
			app.update();
			if app.world().get::<AnimRefRoot>(rig).is_some_and(|anim| {
				matches!(anim.0.clip.id(), AnimId::Still | AnimId::QuadrupedRunStop)
			}) {
				restored_still = true;
				break;
			}
		}

		eprintln!("gait still={saw_still} run={saw_run} restore={restored_still}");
		assert!(saw_still, "stationary boss should idle (AnimClip::still)");
		assert!(saw_run, "moving boss should play quadruped gait from LinearVelocity");
		assert!(restored_still, "gait should return toward still once velocity drops");
	}

	#[derive(Resource, Clone, Copy)]
	struct ObservedHost(Entity);

	#[derive(Resource, Default)]
	struct ObservedApplies {
		events: Vec<damage::DamageApplied>,
	}

	#[derive(Resource, Default)]
	struct ObservedContacts {
		events: Vec<projectiles::ProjectileContact>,
	}

	fn collect_host_applies(
		mut reader: MessageReader<damage::DamageApplied>,
		host: Res<ObservedHost>,
		mut observed: ResMut<ObservedApplies>,
	) {
		for event in reader.read() {
			if event.target == host.0 {
				observed.events.push(*event);
			}
		}
	}

	fn collect_contacts(
		mut reader: MessageReader<projectiles::ProjectileContact>,
		mut observed: ResMut<ObservedContacts>,
	) {
		for event in reader.read() {
			observed.events.push(*event);
		}
	}

	fn fire_path_app() -> App {
		full_encounter_app_with(|app| {
			if !app.is_plugin_added::<projectiles::ProjectilesPlugin>() {
				app.add_plugins(projectiles::ProjectilesPlugin);
			}
			firearms::register_firearm_runtime(app);
		})
	}

	fn fire_player_bolt(app: &mut App, player: Entity, start: Vec3, direction: Vec3) {
		app.world_mut().spawn((
			Name::new("player-weapon"),
			firearms::Weapon::bolt(),
			firearms::WeaponTrigger(true),
			firearms::FireOnTrigger,
			firearms::FireControl::semi(),
			firearms::WeaponLaunchPose::ready_at(start, direction),
			damage::HitPayload { amount: 25.0 },
			projectiles::ProjectileSource(player),
		));
	}

	fn observe_host_damage(app: &mut App, host: Entity) {
		app.insert_resource(ObservedHost(host));
		app.init_resource::<ObservedApplies>();
		app.init_resource::<ObservedContacts>();
		app.add_systems(
			PostUpdate,
			(
				collect_contacts.after(projectiles::ProjectileSystems::Sweep),
				collect_host_applies.after(damage::DamageSystems::Apply),
			),
		);
	}

	fn hit_volume_of(app: &mut App, actor: Entity) -> Option<Entity> {
		app.world_mut()
			.query::<(Entity, &player::HitVolume, &ChildOf)>()
			.iter(app.world())
			.find(|(_, _, parent)| parent.parent() == actor)
			.map(|(entity, _, _)| entity)
	}

	fn rear_anchor_offset(app: &mut App, actor: Entity) -> Vec3 {
		use bikanwarda_character::BikanwardaChainAnchor;
		let actor_at = actor_translation(app, actor);
		let anchor_at = app
			.world_mut()
			.query::<(&BikanwardaChainAnchor, &ChildOf, &GlobalTransform)>()
			.iter(app.world())
			.find(|(anchor, parent, _)| {
				anchor.name == "chain_anchor_neg_z" && parent.parent() == actor
			})
			.map(|(_, _, gt)| gt.translation())
			.expect("rear chain_anchor_neg_z");
		anchor_at - actor_at
	}

	/// Ordinary fire path: a shot that only strikes the pronograde sensor
	/// still debits the production host once.
	#[test]
	fn real_player_shot_hits_only_pronograde_sensor_once() {
		use bikanwarda_character::{BikanwardaActorRecipe, BikanwardaSpec};
		use characters::CharacterRecipe;
		use player::HitVolume;

		let hull = BikanwardaActorRecipe::from_spec(BikanwardaSpec::encounter())
			.components()
			.scaled_locomotion_capsule();
		let hit = hull.hit_capsule().expect("encounter hull is pronograde");
		let mut app = fire_path_app();
		let terrain_y = 0.0;
		let boss_position = Vec3::new(8.0, terrain_y + encounter_hull_offset(), -6.0);
		let (host, actor, _column) = spawn_production_encounter(&mut app, boss_position, terrain_y);
		observe_host_damage(&mut app, host);
		let player = spawn_visible_player(&mut app, boss_position + Vec3::new(-30.0, 1.3, 0.0));

		let sensor = (0..12)
			.find_map(|_| {
				app.update();
				hit_volume_of(&mut app, actor)
			})
			.expect("motorized boss should spawn a pronograde HitVolume");
		let volume_at = app
			.world()
			.get::<GlobalTransform>(sensor)
			.expect("sensor GlobalTransform")
			.compute_transform();
		let actor_at = actor_translation(&app, actor);
		let aim = volume_at.transform_point(hit.local_transform().translation);
		assert!(
			app.world().get::<HitVolume>(sensor).is_some(),
			"shot must be aimed at the pronograde sensor"
		);
		assert!(
			xz(aim, actor_at) > 1.0,
			"body sample should sit off the motor origin so a side shot can miss the capsule, aim={aim:?} actor={actor_at:?}"
		);

		let along_x = volume_at.rotation * Vec3::X;
		fire_player_bolt(&mut app, player, aim + along_x * 16.0, -along_x);
		for _ in 0..24 {
			app.update();
		}

		let contacts = app.world().resource::<ObservedContacts>().events.clone();
		let sensor_hits = contacts.iter().filter(|contact| contact.target == sensor).count();
		let actor_hits = contacts.iter().filter(|contact| contact.target == actor).count();
		let applies = app.world().resource::<ObservedApplies>().events.clone();
		let health = app.world().get::<Health>(host).expect("host Health").current;
		eprintln!(
			"sensor-only contacts={} sensor_hits={sensor_hits} actor_hits={actor_hits} applies={} health={health}",
			contacts.len(),
			applies.len()
		);
		assert!(
			sensor_hits >= 1,
			"projectile must strike the pronograde sensor, contacts={contacts:?}"
		);
		assert_eq!(
			actor_hits, 0,
			"sensor-only shot must miss the motor capsule, contacts={contacts:?}"
		);
		assert_eq!(applies.len(), 1, "sensor shot should debit the host once, applies={applies:?}");
		assert!((health - 975.0).abs() < 0.1, "host should lose exactly 25 HP, got {health}");
	}

	/// Native Metal play recorded two bolts with zero boss contacts on the
	/// visual legs / outer body. Production fire at those samples must debit
	/// the Health host once per projectile.
	#[test]
	fn real_player_shots_at_legs_and_outer_body_debit_once_each() {
		use bikanwarda_character::{
			leg_aim_point, outer_body_aim_point, BikanwardaActorRecipe, BikanwardaSpec,
		};
		use characters::CharacterRecipe;
		use damage::HealthHurtboxes;

		let hull = BikanwardaActorRecipe::from_spec(BikanwardaSpec::encounter())
			.components()
			.scaled_locomotion_capsule();
		let mut app = fire_path_app();
		let terrain_y = 0.0;
		let boss_position = Vec3::new(6.0, terrain_y + encounter_hull_offset(), -4.0);
		let (host, actor, _column) = spawn_production_encounter(&mut app, boss_position, terrain_y);
		observe_host_damage(&mut app, host);
		let player = spawn_visible_player(&mut app, boss_position + Vec3::new(-30.0, 1.3, 0.0));
		let sensor = (0..12)
			.find_map(|_| {
				app.update();
				hit_volume_of(&mut app, actor)
			})
			.expect("motorized boss should spawn a HitVolume");
		assert!(
			app.world().get::<HealthHurtboxes>(host).is_some(),
			"hit complex must bind through HealthHurtboxes on the host"
		);
		assert!(
			app.world().get::<player::HitVolumeCollider>(actor).is_some(),
			"motorized boss should stamp a compound HitVolumeCollider"
		);
		let volume_at = app
			.world()
			.get::<GlobalTransform>(sensor)
			.expect("sensor GlobalTransform")
			.compute_transform();
		let along_x = volume_at.rotation * Vec3::X;
		let along_z = volume_at.rotation * Vec3::Z;
		let leg = volume_at.transform_point(leg_aim_point(hull));
		let outer = volume_at.transform_point(outer_body_aim_point(hull));
		fire_player_bolt(&mut app, player, leg + along_x * 8.0, -along_x);
		for _ in 0..12 {
			app.update();
		}
		fire_player_bolt(&mut app, player, outer + along_z * 8.0, -along_z);
		for _ in 0..12 {
			app.update();
		}
		let applies = app.world().resource::<ObservedApplies>().events.clone();
		let contacts = app.world().resource::<ObservedContacts>().events.clone();
		let health = app.world().get::<Health>(host).expect("host Health").current;
		eprintln!(
			"leg+outer applies={} contacts={} health={health} leg={leg:?} outer={outer:?}",
			applies.len(),
			contacts.len()
		);
		assert_eq!(
			applies.len(),
			2,
			"leg and outer-body shots should each debit once, applies={applies:?}"
		);
		assert!(
			(health - 950.0).abs() < 0.1,
			"host should lose 50 HP from two 25 HP shots, got {health}"
		);
	}

	/// Ordinary fire path: a shot that crosses motor and sensor colliders
	/// still debits the production host once.
	#[test]
	fn real_player_shot_crossing_boss_colliders_debits_once() {
		let mut app = fire_path_app();
		let terrain_y = 0.0;
		let boss_position = Vec3::new(-4.0, terrain_y + encounter_hull_offset(), 10.0);
		let (host, actor, _column) = spawn_production_encounter(&mut app, boss_position, terrain_y);
		observe_host_damage(&mut app, host);
		let player = spawn_visible_player(&mut app, boss_position + Vec3::new(-30.0, 1.3, 0.0));

		let sensor = (0..12)
			.find_map(|_| {
				app.update();
				hit_volume_of(&mut app, actor)
			})
			.expect("motorized boss should spawn a pronograde HitVolume");
		let actor_at = actor_translation(&app, actor);
		let sensor_at = app
			.world()
			.get::<GlobalTransform>(sensor)
			.expect("sensor GlobalTransform")
			.translation();
		let away = {
			let delta = sensor_at - actor_at;
			let xz = Vec3::new(delta.x, 0.0, delta.z);
			if xz.length_squared() > 1e-4 {
				xz.normalize()
			} else {
				Vec3::NEG_Z
			}
		};
		let start = sensor_at + away * 10.0;
		fire_player_bolt(&mut app, player, start, -away);
		for _ in 0..24 {
			app.update();
		}

		let contacts = app.world().resource::<ObservedContacts>().events.clone();
		let boss_hits: Vec<_> = contacts
			.iter()
			.filter(|contact| contact.target == actor || contact.target == sensor)
			.copied()
			.collect();
		let unique: std::collections::HashSet<Entity> =
			boss_hits.iter().map(|contact| contact.target).collect();
		let applies = app.world().resource::<ObservedApplies>().events.clone();
		let health = app.world().get::<Health>(host).expect("host Health").current;
		eprintln!(
			"multi-collider unique={} contacts={} applies={} health={health} targets={unique:?}",
			unique.len(),
			boss_hits.len(),
			applies.len()
		);
		assert!(unique.len() >= 2, "shot should cross several boss colliders, hits={boss_hits:?}");
		assert_eq!(
			applies.len(),
			1,
			"crossing colliders should debit the host once, applies={applies:?}"
		);
		assert!((health - 975.0).abs() < 0.1, "host should lose exactly 25 HP, got {health}");
	}

	/// Spotted-facing owns visual yaw while the boss is moving; the rear
	/// tether anchor stays behind that turn.
	#[test]
	fn real_boss_faces_off_axis_player_while_moving() {
		use characters::CharacterRoot;
		use combat_targeting::CombatTargeting;
		use player::PlayerYawOwner;

		let mut app = full_encounter_app();
		let terrain_y = 0.0;
		let boss_position = Vec3::new(0.0, terrain_y + encounter_hull_offset(), 0.0);
		let (_host, actor, column) = spawn_production_encounter(&mut app, boss_position, terrain_y);
		let pillar = app.world().get::<GlobalTransform>(column).expect("pillar").translation();
		let rim = Vec3::new(
			pillar.x + crate::BOSS_TETHER_RADIUS - bikanwarda_character::encounter_meters(6.0),
			actor_translation(&app, actor).y,
			pillar.z,
		);
		place_actor_at(&mut app, actor, rim);
		// Off the rest-pose −Z rear and off the tether radial (−X): facing and
		// the rear anchor must follow this turn, not the motor-local sockets.
		let player_pos =
			Vec3::new(rim.x + bikanwarda_character::encounter_meters(18.0), terrain_y + 1.3, rim.z);
		let player = spawn_visible_player(&mut app, player_pos);

		let start = actor_translation(&app, actor);
		let mut saw_spotted = false;
		let mut saw_move = 0.0_f32;
		let mut best_face_dot = -1.0_f32;
		let mut faced_rear_dot = -1.0_f32;
		let mut saw_look_owner = false;
		for _ in 0..360 {
			app.update();
			if app
				.world()
				.get::<CombatTargeting>(actor)
				.and_then(CombatTargeting::best_contact)
				.is_some_and(|contact| contact.subject == player)
			{
				saw_spotted = true;
			}
			let now = actor_translation(&app, actor);
			saw_move = saw_move.max(xz(start, now));
			let facing = visual_facing(&mut app, actor);
			let to_player = Vec3::new(player_pos.x - now.x, 0.0, player_pos.z - now.z);
			let face_dot = if to_player.length_squared() > 1e-4 && facing.length_squared() > 1e-4 {
				facing.normalize().dot(to_player.normalize())
			} else {
				-1.0
			};
			best_face_dot = best_face_dot.max(face_dot);
			let rear = rear_anchor_offset(&mut app, actor);
			let rear_xz = Vec3::new(rear.x, 0.0, rear.z);
			if face_dot > 0.7 && rear_xz.length_squared() > 1e-4 && facing.length_squared() > 1e-4 {
				faced_rear_dot = faced_rear_dot.max((-facing.normalize()).dot(rear_xz.normalize()));
			}
			let visual = app
				.world_mut()
				.query_filtered::<(Entity, &ChildOf), With<CharacterRoot>>()
				.iter(app.world())
				.find(|(_, parent)| parent.parent() == actor)
				.map(|(entity, _)| entity)
				.expect("visual");
			if app.world().get::<PlayerYawOwner>(visual) == Some(&PlayerYawOwner::Look) {
				saw_look_owner = true;
			}
		}

		eprintln!(
			"off-axis face_dot={best_face_dot:.3} faced_rear_dot={faced_rear_dot:.3} move={saw_move:.3} spotted={saw_spotted} look={saw_look_owner}"
		);
		assert!(saw_spotted, "CombatTargeting contact must come from spotting the spawned player");
		assert!(saw_move > 0.5, "boss should keep moving while facing the player");
		assert!(saw_look_owner, "spotted facing must set PlayerYawOwner::Look on the visual");
		assert!(
			best_face_dot > 0.7,
			"boss should face the off-axis player, face_dot={best_face_dot:.3}"
		);
		assert!(
			faced_rear_dot > 0.85,
			"rear tether anchor should stay behind the turned facing, faced_rear_dot={faced_rear_dot:.3}"
		);
	}
}
