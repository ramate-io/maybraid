//! Integration tests for homeostatic mob behavior.
//! Tests the core homeostatic logic without full mob scene spawning.

use bevy::prelude::*;

use crate::homeostatic::{
	HomeostaticCellIndex, HomeostaticMob, HomeostaticMobConfig, HomeostaticRetreat,
	PlayerHomeostaticCell,
};
use crate::plugin::HomeostaticMobsPlugin;

/// Test marker for the player entity
#[derive(Component, Clone, Copy, Debug)]
struct TestPlayer;

/// Test app with just homeostatic systems (no full mob scene spawning)
fn test_app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_plugins(HomeostaticMobsPlugin::<TestPlayer>::default());
	app
}

#[test]
fn deterministic_flagging_hits_target_fraction() {
	let target_fraction = 0.33;
	let sample_size = 10_000;

	let mut flagged_count = 0;
	for ix in -50..50 {
		for iz in -50..50 {
			let cell = HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(target_fraction) {
				flagged_count += 1;
			}
		}
	}

	let actual_fraction = flagged_count as f64 / sample_size as f64;
	let error = (actual_fraction - target_fraction).abs();

	assert!(
		error < 0.02,
		"expected ~{target_fraction}, got {actual_fraction} (error {error})"
	);
}

#[test]
fn deterministic_flagging_is_stable() {
	let cell = HomeostaticCellIndex(5, 10);
	let result1 = cell.should_host_mob(0.33);
	let result2 = cell.should_host_mob(0.33);
	assert_eq!(result1, result2, "same cell gives same result");
}

// NOTE: player_cell_resource_tracks_movement removed - needs full mob infrastructure

#[test]
fn mob_enters_retreat_near_boundary() {
	let mut app = test_app();
	let config = app.world().resource::<HomeostaticMobConfig>().clone();

	let home_cell = HomeostaticCellIndex(0, 0);
	let home_center = home_cell.center(config.cell_size);

	// Spawn player in home cell
	let player = app
		.world_mut()
		.spawn((TestPlayer, Transform::from_translation(home_center)))
		.id();

	// Manually spawn a homeostatic mob (simulating what track_player_homeostatic_cell would do)
	let generation = app.world().resource::<PlayerHomeostaticCell>().generation;
	let mob_entity = Entity::PLACEHOLDER;
	let mob = app
		.world_mut()
		.spawn((
			HomeostaticMob { home_cell, generation, mob_entity },
			Transform::from_translation(home_center),
			GlobalTransform::from_translation(home_center),
		))
		.id();

	// Update the mob_entity field to point to itself
	app.world_mut()
		.entity_mut(mob)
		.get_mut::<HomeostaticMob>()
		.unwrap()
		.mob_entity = mob;

	app.update();

	// Mob should not be in retreat yet
	assert!(app.world().get::<HomeostaticRetreat>(mob).is_none());

	// Move player near the edge of the 3×3 neighborhood
	let retreat_margin = config.retreat_margin;
	let edge_position =
		home_center + Vec3::new(config.cell_size * 1.5 - retreat_margin + 10.0, 0.0, 0.0);

	app.world_mut()
		.entity_mut(player)
		.get_mut::<Transform>()
		.unwrap()
		.translation = edge_position;

	app.update();

	// Mob should now be in retreat mode
	let retreat = app
		.world()
		.get::<HomeostaticRetreat>(mob)
		.expect("mob should be in retreat");

	// Check retreat target is reasonable (toward home, away from player edge)
	let mob_pos = app.world().get::<Transform>(mob).unwrap().translation;
	let to_home = home_center - mob_pos;
	let to_retreat = retreat.target - mob_pos;

	// Retreat target should have positive component toward home (allow for zero if already at home)
	let retreat_len = to_retreat.length();
	let home_len = to_home.length();
	if retreat_len > 0.1 && home_len > 0.1 {
		let retreat_dot_home = to_retreat.normalize().dot(to_home.normalize());
		assert!(
			retreat_dot_home >= -0.1,
			"retreat should not be strongly away from home (dot = {retreat_dot_home})"
		);
	}

	// Check that a POI goal was set for retreat
	let has_poi_goal = app.world().get::<poi_intelligence::PoiGoal>(mob).is_some();
	assert!(
		has_poi_goal,
		"retreating mob should have POI goal to drive movement"
	);
}

#[test]
fn mob_despawns_outside_neighborhood() {
	let mut app = test_app();
	let config = app.world().resource::<HomeostaticMobConfig>().clone();

	let home_cell = HomeostaticCellIndex(0, 0);
	let home_center = home_cell.center(config.cell_size);

	// Spawn player in home cell
	let player = app
		.world_mut()
		.spawn((TestPlayer, Transform::from_translation(home_center)))
		.id();

	// Spawn homeostatic mob
	let generation = app.world().resource::<PlayerHomeostaticCell>().generation;
	let mob_entity = Entity::PLACEHOLDER;
	let mob = app
		.world_mut()
		.spawn((
			HomeostaticMob { home_cell, generation, mob_entity },
			Transform::from_translation(home_center),
		))
		.id();

	app.world_mut()
		.entity_mut(mob)
		.get_mut::<HomeostaticMob>()
		.unwrap()
		.mob_entity = mob;

	app.update();

	// Mob should exist
	assert!(app.world().get_entity(mob).is_ok());

	// Move player far away (outside 3×3 neighborhood)
	let far_away = home_center + Vec3::new(1000.0, 0.0, 1000.0);
	app.world_mut()
		.entity_mut(player)
		.get_mut::<Transform>()
		.unwrap()
		.translation = far_away;

	app.update();

	// Mob should be despawned
	assert!(
		app.world().get_entity(mob).is_err(),
		"mob should despawn when player leaves 3×3 neighborhood"
	);
}

#[test]
fn mob_persists_while_player_in_neighborhood() {
	let mut app = test_app();
	let config = app.world().resource::<HomeostaticMobConfig>().clone();

	let home_cell = HomeostaticCellIndex(0, 0);
	let home_center = home_cell.center(config.cell_size);

	// Spawn player in home cell
	let player = app
		.world_mut()
		.spawn((TestPlayer, Transform::from_translation(home_center)))
		.id();

	// Spawn homeostatic mob
	let generation = app.world().resource::<PlayerHomeostaticCell>().generation;
	let mob_entity = Entity::PLACEHOLDER;
	let mob = app
		.world_mut()
		.spawn((
			HomeostaticMob { home_cell, generation, mob_entity },
			Transform::from_translation(home_center),
		))
		.id();

	app.world_mut()
		.entity_mut(mob)
		.get_mut::<HomeostaticMob>()
		.unwrap()
		.mob_entity = mob;

	app.update();

	// Move player within 3×3 neighborhood (one cell over)
	let neighbor_cell = HomeostaticCellIndex(1, 0);
	let neighbor_center = neighbor_cell.center(config.cell_size);

	app.world_mut()
		.entity_mut(player)
		.get_mut::<Transform>()
		.unwrap()
		.translation = neighbor_center;

	app.update();

	// Mob should still exist
	assert!(
		app.world().get_entity(mob).is_ok(),
		"mob should persist when player moves within 3×3 neighborhood"
	);
}

#[test]
fn retreat_moves_toward_home_and_away_from_player() {
	let mut app = test_app();
	let config = app.world().resource::<HomeostaticMobConfig>().clone();

	let home_cell = HomeostaticCellIndex(0, 0);
	let home_center = home_cell.center(config.cell_size);

	// Spawn player approaching from the right (positive x)
	let player_pos = home_center + Vec3::new(config.cell_size * 1.4, 0.0, 0.0);
	app.world_mut()
		.spawn((TestPlayer, Transform::from_translation(player_pos)));

	// Spawn mob between home center and player
	let generation = app.world().resource::<PlayerHomeostaticCell>().generation;
	let mob_start = home_center + Vec3::new(config.cell_size * 0.3, 0.0, 0.0);
	let mob_entity = Entity::PLACEHOLDER;
	let mob = app
		.world_mut()
		.spawn((
			HomeostaticMob { home_cell, generation, mob_entity },
			Transform::from_translation(mob_start),
			GlobalTransform::from_translation(mob_start),
		))
		.id();

	app.world_mut()
		.entity_mut(mob)
		.get_mut::<HomeostaticMob>()
		.unwrap()
		.mob_entity = mob;

	// Trigger retreat
	app.update();

	let retreat = app
		.world()
		.get::<HomeostaticRetreat>(mob)
		.expect("mob should be in retreat");

	// Retreat target should be left of mob (toward home, away from player)
	assert!(
		retreat.target.x < mob_start.x,
		"retreat target should be left (toward home, away from player): target.x={}, mob.x={}",
		retreat.target.x,
		mob_start.x
	);

	// Check retreat is toward home side
	let home_dir = (home_center - mob_start).normalize_or_zero();
	let retreat_dir = (retreat.target - mob_start).normalize_or_zero();
	let dot = home_dir.dot(retreat_dir);

	assert!(
		dot > 0.5,
		"retreat should be strongly toward home (dot = {dot})"
	);
}

// NOTE: generation_increments_when_changing_cells and generation_does_not_increment_within_same_cell
// removed - need full mob infrastructure for track_player_homeostatic_cell system
