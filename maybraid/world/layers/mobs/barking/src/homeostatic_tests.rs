//! Integration tests for homeostatic mob behavior using real world infrastructure.

use std::time::Duration;

use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use chico::ChicoPresentationPlugin;
use durham::{
	fine_patch_cell_layout, Durham, DurhamWindow, TerrainConfig, TerrainMeshAssets,
	TerrainMeshLodBand, TerrainStampConfigs, WaterMeshAssets, WaterPresentationPlugin,
	WatershedConfigs,
};
use lod::hcsg::{settle_presentation, HcsgRestartRequest, SettlePresentation};
use lod::lod_ref::LodNodePose;
use lod::LodViewer;
use mob_scenes::MobScene;
use richmond::{
	AuthoredDevelopment, AuthoredDevelopments, BuiltPresentationPlugin, DevelopmentConfig,
	DevelopmentKind, DevelopmentSites,
};
use terrain_layer_model::OnTerrain;
use urbanization_cells::UrbanizationSelection;
use urbanization_layer_model::Urbanization;

use crate::homeostatic::{
	HomeostaticCellIndex, HomeostaticMob, HomeostaticMobConfig, HomeostaticRetreat,
	PlayerHomeostaticCell,
};
use crate::plugin::HomeostaticMobsPlugin;
use crate::{BarkingPresentationPlugin, MobNeighborhood};

/// Test marker for the player entity
#[derive(Component, Clone, Copy, Debug)]
struct TestPlayer;

type Ground = OnTerrain<Durham>;
type Urban = Urbanization<richmond::Richmond<Ground>>;

const IDLE: Duration = Duration::from_secs(300);

/// The 2×2 fine patch's footprint.
const PATCH: f32 = 160.0;

fn les_halles() -> AuthoredDevelopment {
	AuthoredDevelopment {
		cell: bevy::shape::Aabb3d::from_min_max(
			Vec3::new(10.0, 0.0, 10.0),
			Vec3::new(150.0, 1.0, 150.0),
		),
		kinds: vec![DevelopmentKind::LesHalles],
		height: 12.0,
		config: DevelopmentConfig::default(),
		courtyard: None,
	}
}

/// Create a test app with real world infrastructure.
fn test_app() -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, StatesPlugin))
		.add_plugins((AssetPlugin::default(), ScenePlugin))
		.init_asset::<Mesh>()
		.init_asset::<StandardMaterial>()
		.init_asset::<bevy::world_serialization::WorldAsset>()
		.insert_resource(fine_patch_cell_layout(1, IVec2::new(-1, -1)))
		.insert_resource(TerrainStampConfigs::from_world_seed(1))
		.insert_resource(WatershedConfigs::default().with_seed(1))
		.insert_resource(TerrainMeshAssets {
			config: TerrainConfig::new(1),
			material: Handle::default(),
			lod_bands: vec![TerrainMeshLodBand { max_radius_cells: 1, res_2: 2 }],
			outer_add_walls: false,
			fine_grid_max_radius: Some(1),
			macro_seam_half_extents: Vec::new(),
			macro_cell_min_size: None,
			macro_res_2: None,
		})
		.insert_resource(WaterMeshAssets { material: Handle::default() })
		.insert_resource(DevelopmentConfig {
			sites: DevelopmentSites::Authored,
			..DevelopmentConfig::default()
		})
		.insert_resource(AuthoredDevelopments(vec![les_halles()]))
		.init_resource::<chico::ForestSelection>()
		.insert_resource(HcsgRestartRequest::queued())
		.insert_resource(UrbanizationSelection::default())
		.add_plugins((
			lod::hcsg::HcsgBoundsPlugin::<MobNeighborhood>::default(),
			WaterPresentationPlugin::<DurhamWindow>::default(),
			BuiltPresentationPlugin::<MobNeighborhood, Ground>::default(),
			ChicoPresentationPlugin::<MobNeighborhood, Urban>::default(),
			BarkingPresentationPlugin::<MobNeighborhood, Urban>::default(),
			crate::MobScenePresentationPlugin,
			HomeostaticMobsPlugin::<TestPlayer>::default(),
		));
	
	durham::register_durham_hcsg_session(&mut app);
	app.finish();
	app.cleanup();
	
	let at = Transform::IDENTITY;
	app.world_mut()
		.spawn((LodViewer, at, LodNodePose { previous: at, current: at }));
	
	app
}

fn settle(app: &mut App) -> anyhow::Result<()> {
	settle_presentation(app, SettlePresentation::layer_integration(IDLE)).map_err(Into::into)
}

fn mob_hosts(app: &mut App) -> usize {
	app.world_mut()
		.query_filtered::<(), (With<MobScene>, Allow<Disabled>)>()
		.iter(app.world())
		.count()
}

fn wait_for_homeostatic_mobs(app: &mut App, expected: usize) {
	for _ in 0..200 {
		let count = app
			.world_mut()
			.query::<&HomeostaticMob>()
			.iter(app.world())
			.count();
		if count >= expected {
			return;
		}
		app.update();
	}
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
fn entering_flagged_cell_activates_one_mob() -> anyhow::Result<()> {
	let mut app = test_app();
	settle(&mut app)?;

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
	wait_for_homeostatic_mobs(&mut app, 1);

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
	Ok(())
}

#[test]
fn standing_in_neighbor_cell_does_not_activate() -> anyhow::Result<()> {
	let mut app = test_app();
	settle(&mut app)?;

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
	for _ in 0..50 {
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
	Ok(())
}

#[test]
fn mob_persists_while_player_in_3x3_neighborhood() -> anyhow::Result<()> {
	let mut app = test_app();
	settle(&mut app)?;

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
	wait_for_homeostatic_mobs(&mut app, 1);

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
	for _ in 0..50 {
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
	Ok(())
}

#[test]
fn mob_enters_retreat_and_moves_away() -> anyhow::Result<()> {
	let mut app = test_app();
	settle(&mut app)?;

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
	wait_for_homeostatic_mobs(&mut app, 1);

	// Get mob entity and add GlobalTransform
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
	for _ in 0..50 {
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
	Ok(())
}

#[test]
fn mob_despawns_outside_neighborhood() -> anyhow::Result<()> {
	let mut app = test_app();
	settle(&mut app)?;

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
	wait_for_homeostatic_mobs(&mut app, 1);

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
	for _ in 0..50 {
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
	Ok(())
}

#[test]
fn reentering_cell_reactivates_mob_without_duplicates() -> anyhow::Result<()> {
	let mut app = test_app();
	settle(&mut app)?;

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
	wait_for_homeostatic_mobs(&mut app, 1);

	let original_generation = app.world().resource::<PlayerHomeostaticCell>().generation;

	// Move player far away
	let far_away = cell_center + Vec3::new(1000.0, 0.0, 1000.0);
	if let Some(mut player_transform) = app.world_mut().get_mut::<Transform>(player) {
		player_transform.translation = far_away;
	}

	for _ in 0..50 {
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

	wait_for_homeostatic_mobs(&mut app, 1);

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
	Ok(())
}

#[test]
fn existing_mob_layers_unaffected() -> anyhow::Result<()> {
	// This test verifies that homeostatic mobs don't interfere with existing HCSG layers
	let mut app = test_app();
	settle(&mut app)?;

	// Count existing HCSG mobs
	let hcsg_mobs_before = mob_hosts(&mut app);

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

		wait_for_homeostatic_mobs(&mut app, 1);

		// HCSG mobs should be unaffected
		let hcsg_mobs_after = mob_hosts(&mut app);
		assert_eq!(
			hcsg_mobs_before, hcsg_mobs_after,
			"HCSG mob count should remain unchanged"
		);

		// Should have homeostatic mobs active
		let homeostatic_count = app
			.world_mut()
			.query::<&HomeostaticMob>()
			.iter(app.world())
			.count();
		assert_eq!(homeostatic_count, 1, "should have one homeostatic mob");
	}

	Ok(())
}
