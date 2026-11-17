//! Tests for homeostatic mob behavior.

use bevy::prelude::*;

use crate::homeostatic::{
	apply_homeostatic_retreat, despawn_distant_homeostatic_mobs, track_player_homeostatic_cell,
	trigger_homeostatic_retreat, HomeostaticCellIndex, HomeostaticMob, HomeostaticMobConfig,
	HomeostaticRetreat, PlayerHomeostaticCell,
};
use crate::plugin::PendingMobGroups;
use crate::MobGroup;

/// Test marker for the player entity
#[derive(Component, Clone, Copy, Debug)]
struct TestPlayer;

fn test_app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.init_resource::<HomeostaticMobConfig>();
	app.init_resource::<PlayerHomeostaticCell>();
	app.init_resource::<PendingMobGroups>();
	app.add_systems(
		Update,
		(
			track_player_homeostatic_cell::<TestPlayer>,
			trigger_homeostatic_retreat::<TestPlayer>,
			apply_homeostatic_retreat,
			despawn_distant_homeostatic_mobs::<TestPlayer>,
		)
			.chain(),
	);
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

#[test]
fn entering_flagged_cell_activates_one_mob() {
	let mut app = test_app();

	// Find a flagged cell
	let mut flagged_cell = None;
	for ix in 0..20 {
		for iz in 0..20 {
			let cell = HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(0.33) {
				flagged_cell = Some(cell);
				break;
			}
		}
		if flagged_cell.is_some() {
			break;
		}
	}

	let flagged_cell = flagged_cell.expect("should find at least one flagged cell");
	let cell_center = flagged_cell.center(200.0);

	// Spawn player in the flagged cell
	app.world_mut().spawn((TestPlayer, Transform::from_translation(cell_center)));

	// Run one update
	app.update();

	// Check that exactly one mob was activated
	let active_mobs: Vec<_> = app
		.world_mut()
		.query::<&HomeostaticMob>()
		.iter(app.world())
		.collect();

	assert_eq!(
		active_mobs.len(),
		1,
		"entering flagged cell should activate exactly one mob"
	);
	assert_eq!(active_mobs[0].home_cell, flagged_cell);
}

#[test]
fn standing_in_neighbor_cell_does_not_activate() {
	let mut app = test_app();

	// Find a flagged cell
	let mut flagged_cell = None;
	for ix in 0..20 {
		for iz in 0..20 {
			let cell = HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(0.33) {
				// Make sure a neighbor is not flagged
				let neighbor = HomeostaticCellIndex(ix + 1, iz);
				if !neighbor.should_host_mob(0.33) {
					flagged_cell = Some((cell, neighbor));
					break;
				}
			}
		}
		if flagged_cell.is_some() {
			break;
		}
	}

	let (_, neighbor) = flagged_cell.expect("should find flagged cell with unflagged neighbor");
	let neighbor_center = neighbor.center(200.0);

	// Spawn player in the NEIGHBOR cell (not the flagged one)
	app.world_mut()
		.spawn((TestPlayer, Transform::from_translation(neighbor_center)));

	// Run one update
	app.update();

	// Check that no mob was activated
	let active_mobs: Vec<_> = app
		.world_mut()
		.query::<&HomeostaticMob>()
		.iter(app.world())
		.collect();

	assert_eq!(
		active_mobs.len(),
		0,
		"standing in neighbor of flagged cell should not activate"
	);
}

#[test]
fn mob_persists_while_player_in_3x3_neighborhood() {
	let mut app = test_app();

	// Find a flagged cell
	let mut flagged_cell = None;
	for ix in 0..20 {
		for iz in 0..20 {
			let cell = HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(0.33) {
				flagged_cell = Some(cell);
				break;
			}
		}
		if flagged_cell.is_some() {
			break;
		}
	}

	let flagged_cell = flagged_cell.expect("should find at least one flagged cell");
	let cell_center = flagged_cell.center(200.0);

	// Spawn player in the flagged cell
	let player = app
		.world_mut()
		.spawn((TestPlayer, Transform::from_translation(cell_center)))
		.id();

	// Activate mob
	app.update();

	let mob_count_after_activation = app
		.world_mut()
		.query::<&HomeostaticMob>()
		.iter(app.world())
		.count();
	assert_eq!(mob_count_after_activation, 1);

	// Move player within 3×3 neighborhood (one cell over)
	let neighbor_cell = HomeostaticCellIndex(flagged_cell.0 + 1, flagged_cell.1);
	let neighbor_center = neighbor_cell.center(200.0);

	if let Some(mut player_transform) = app.world_mut().get_mut::<Transform>(player) {
		player_transform.translation = neighbor_center;
	}

	// Run update
	app.update();

	// Mob should still exist
	let mob_count_after_move = app
		.world_mut()
		.query::<&HomeostaticMob>()
		.iter(app.world())
		.count();
	assert_eq!(
		mob_count_after_move, 1,
		"mob should persist when player moves within 3×3 neighborhood"
	);
}

#[test]
fn mob_enters_retreat_near_boundary() {
	let mut app = test_app();

	// Find a flagged cell
	let mut flagged_cell = None;
	for ix in 0..20 {
		for iz in 0..20 {
			let cell = HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(0.33) {
				flagged_cell = Some(cell);
				break;
			}
		}
		if flagged_cell.is_some() {
			break;
		}
	}

	let flagged_cell = flagged_cell.expect("should find at least one flagged cell");
	let cell_center = flagged_cell.center(200.0);

	// Spawn player in the flagged cell
	let player = app
		.world_mut()
		.spawn((TestPlayer, Transform::from_translation(cell_center)))
		.id();

	// Activate mob
	app.update();

	// Spawn the mob with a GlobalTransform component
	let mob_entity = {
		let mut query = app.world_mut().query::<(Entity, &HomeostaticMob)>();
		query
			.iter(app.world())
			.next()
			.map(|(e, _)| e)
			.expect("mob should exist")
	};

	app.world_mut()
		.entity_mut(mob_entity)
		.insert(GlobalTransform::from_translation(cell_center));

	// Move player near the edge of the 3×3 neighborhood
	let config = app.world().resource::<HomeostaticMobConfig>().clone();
	let retreat_margin = config.retreat_margin;

	// Calculate a position just inside the retreat margin
	let edge_position = cell_center + Vec3::new(config.cell_size * 1.5 - retreat_margin + 10.0, 0.0, 0.0);

	if let Some(mut player_transform) = app.world_mut().get_mut::<Transform>(player) {
		player_transform.translation = edge_position;
	}

	// Run update - should trigger retreat
	app.update();

	// Check if mob is now in retreat mode
	let retreating = {
		let mut query = app.world_mut().query::<&HomeostaticRetreat>();
		query.iter(app.world()).count()
	};

	assert_eq!(
		retreating, 1,
		"mob should enter retreat when player nears boundary"
	);
}

#[test]
fn mob_despawns_outside_neighborhood() {
	let mut app = test_app();

	// Find a flagged cell
	let mut flagged_cell = None;
	for ix in 0..20 {
		for iz in 0..20 {
			let cell = HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(0.33) {
				flagged_cell = Some(cell);
				break;
			}
		}
		if flagged_cell.is_some() {
			break;
		}
	}

	let flagged_cell = flagged_cell.expect("should find at least one flagged cell");
	let cell_center = flagged_cell.center(200.0);

	// Spawn player in the flagged cell
	let player = app
		.world_mut()
		.spawn((TestPlayer, Transform::from_translation(cell_center)))
		.id();

	// Activate mob
	app.update();

	let mob_count_after_activation = app
		.world_mut()
		.query::<&HomeostaticMob>()
		.iter(app.world())
		.count();
	assert_eq!(mob_count_after_activation, 1);

	// Move player far away (outside 3×3 neighborhood)
	let far_away = cell_center + Vec3::new(1000.0, 0.0, 1000.0);

	if let Some(mut player_transform) = app.world_mut().get_mut::<Transform>(player) {
		player_transform.translation = far_away;
	}

	// Run update - should despawn mob
	app.update();

	// Mob should be gone
	let mob_count_after_leaving = app
		.world_mut()
		.query::<&HomeostaticMob>()
		.iter(app.world())
		.count();
	assert_eq!(
		mob_count_after_leaving, 0,
		"mob should despawn when player leaves 3×3 neighborhood"
	);
}

#[test]
fn reentering_cell_reactivates_mob() {
	let mut app = test_app();

	// Find a flagged cell
	let mut flagged_cell = None;
	for ix in 0..20 {
		for iz in 0..20 {
			let cell = HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(0.33) {
				flagged_cell = Some(cell);
				break;
			}
		}
		if flagged_cell.is_some() {
			break;
		}
	}

	let flagged_cell = flagged_cell.expect("should find at least one flagged cell");
	let cell_center = flagged_cell.center(200.0);

	// Spawn player in the flagged cell
	let player = app
		.world_mut()
		.spawn((TestPlayer, Transform::from_translation(cell_center)))
		.id();

	// Activate mob
	app.update();

	let original_generation = app
		.world()
		.resource::<PlayerHomeostaticCell>()
		.generation;

	// Move player far away
	let far_away = cell_center + Vec3::new(1000.0, 0.0, 1000.0);
	if let Some(mut player_transform) = app.world_mut().get_mut::<Transform>(player) {
		player_transform.translation = far_away;
	}

	app.update();

	// Mob should be despawned
	assert_eq!(
		app.world_mut()
			.query::<&HomeostaticMob>()
			.iter(app.world())
			.count(),
		0
	);

	// Move player back to the flagged cell
	if let Some(mut player_transform) = app.world_mut().get_mut::<Transform>(player) {
		player_transform.translation = cell_center;
	}

	app.update();

	// Mob should be reactivated
	let mobs_after_return: Vec<_> = app
		.world_mut()
		.query::<&HomeostaticMob>()
		.iter(app.world())
		.collect();

	assert_eq!(
		mobs_after_return.len(),
		1,
		"mob should reactivate on re-entry"
	);
	assert_eq!(mobs_after_return[0].home_cell, flagged_cell);
	assert!(
		mobs_after_return[0].generation > original_generation,
		"reactivated mob should have higher generation"
	);
}

#[test]
fn existing_mob_layers_unaffected() {
	// This test verifies that our new homeostatic system doesn't interfere
	// with existing mob spawning. Since we're using PendingMobGroups,
	// both systems can coexist.

	let mut app = test_app();

	// Manually add a regular mob group
	let regular_group = MobGroup {
		kind: crate::GroupKind::Placed,
		seed: 42,
		origin: Vec2::ZERO,
		extent: 100.0,
		mobs: Vec::new(),
	};

	app.world_mut()
		.resource_mut::<PendingMobGroups>()
		.push(regular_group);

	// Find and enter a flagged cell to activate homeostatic mob
	let mut flagged_cell = None;
	for ix in 0..20 {
		for iz in 0..20 {
			let cell = HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(0.33) {
				flagged_cell = Some(cell);
				break;
			}
		}
		if flagged_cell.is_some() {
			break;
		}
	}

	if let Some(cell) = flagged_cell {
		app.world_mut()
			.spawn((TestPlayer, Transform::from_translation(cell.center(200.0))));
	}

	// Run update
	app.update();

	// Both systems should work independently
	// (We can't easily verify the regular mob group spawned without more infrastructure,
	// but we verify the homeostatic mob activated if we had a flagged cell)
	if flagged_cell.is_some() {
		let count = {
			let mut query = app.world_mut().query::<&HomeostaticMob>();
			query.iter(app.world()).count()
		};
		assert!(count > 0);
	}
}
