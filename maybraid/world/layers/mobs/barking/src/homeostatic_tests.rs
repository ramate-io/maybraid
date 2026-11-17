//! Integration tests for homeostatic mob behavior.
//! Tests the core homeostatic logic without full world infrastructure.

use bevy::prelude::*;
use bevy::scene::ScenePlugin;

use crate::homeostatic::{
	HomeostaticCellIndex, HomeostaticMob, HomeostaticMobConfig, HomeostaticRetreat,
	PlayerHomeostaticCell,
};
use crate::plugin::HomeostaticMobsPlugin;

/// Test marker for the player entity
#[derive(Component, Clone, Copy, Debug)]
struct TestPlayer;

/// Minimal test app with homeostatic plugin and required scene infrastructure
fn test_app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_plugins((AssetPlugin::default(), ScenePlugin))
		.init_asset::<Mesh>()
		.init_asset::<StandardMaterial>()
		.init_asset::<bevy::world_serialization::WorldAsset>()
		.add_plugins(crate::MobScenePresentationPlugin)
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
	app.world_mut()
		.spawn((TestPlayer, Transform::from_translation(cell_center)));

	// Run updates to allow activation
	for _ in 0..10 {
		app.update();
	}

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

	// Find a flagged cell with unflagged neighbor
	let mut test_cells = None;
	for ix in 0..20 {
		for iz in 0..20 {
			let cell = HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(0.33) {
				let neighbor = HomeostaticCellIndex(ix + 1, iz);
				if !neighbor.should_host_mob(0.33) {
					test_cells = Some((cell, neighbor));
					break;
				}
			}
		}
		if test_cells.is_some() {
			break;
		}
	}

	let (_, neighbor) = test_cells.expect("should find flagged cell with unflagged neighbor");
	let neighbor_center = neighbor.center(200.0);

	// Spawn player in the NEIGHBOR cell (not the flagged one)
	app.world_mut()
		.spawn((TestPlayer, Transform::from_translation(neighbor_center)));

	// Run updates
	for _ in 0..10 {
		app.update();
	}

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
	for _ in 0..10 {
		app.update();
	}

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

	// Run updates
	for _ in 0..10 {
		app.update();
	}

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
	for _ in 0..10 {
		app.update();
	}

	// Get mob entity and add GlobalTransform (needed for retreat trigger)
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
	let edge_position =
		cell_center + Vec3::new(config.cell_size * 1.5 - retreat_margin + 10.0, 0.0, 0.0);

	if let Some(mut player_transform) = app.world_mut().get_mut::<Transform>(player) {
		player_transform.translation = edge_position;
	}

	// Run updates - should trigger retreat
	for _ in 0..10 {
		app.update();
	}

	// Check if mob is now in retreat mode
	let retreating = {
		let mut query = app.world_mut().query::<&HomeostaticRetreat>();
		query.iter(app.world()).count()
	};

	assert_eq!(
		retreating, 1,
		"mob should enter retreat when player nears boundary"
	);

	// Check that a POI goal was set for retreat
	let has_poi_goal = {
		let mut query = app.world_mut().query::<&poi_intelligence::PoiGoal>();
		query.get(app.world(), mob_entity).is_ok()
	};

	assert!(
		has_poi_goal,
		"retreating mob should have POI goal to drive movement"
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
	for _ in 0..10 {
		app.update();
	}

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

	// Run updates - should despawn mob
	for _ in 0..10 {
		app.update();
	}

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
fn reentering_cell_reactivates_mob_without_duplicates() {
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
	for _ in 0..10 {
		app.update();
	}

	let original_generation = app.world().resource::<PlayerHomeostaticCell>().generation;

	// Move player far away
	let far_away = cell_center + Vec3::new(1000.0, 0.0, 1000.0);
	if let Some(mut player_transform) = app.world_mut().get_mut::<Transform>(player) {
		player_transform.translation = far_away;
	}

	for _ in 0..10 {
		app.update();
	}

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

	for _ in 0..10 {
		app.update();
	}

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
	// This test verifies that homeostatic systems can run without interfering
	// with other mob systems. Since we don't have full HCSG setup here,
	// we just verify the homeostatic plugin initializes correctly.
	let mut app = test_app();

	// Find and enter a flagged cell
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

		for _ in 0..10 {
			app.update();
		}

		// Should have homeostatic mob active
		let homeostatic_count = app
			.world_mut()
			.query::<&HomeostaticMob>()
			.iter(app.world())
			.count();
		assert_eq!(homeostatic_count, 1, "should have one homeostatic mob");
	}
}
