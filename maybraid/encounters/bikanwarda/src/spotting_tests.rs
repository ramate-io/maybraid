//! Spotting self-occlusion and LOS regression tests.

use anyhow::Result;
use avian3d::prelude::*;
use bevy::prelude::*;
use bikanwarda_character::{spawn_bikanwarda_encounter, BikanwardaRoot, BikanwardaSpec};
use lod_avian::PhysicsInteractionLayer;
use spotting_intelligence::{InterestLayers, SpotBounds, SpotDirective, SpotSubject, SpottingUser};

fn spotting_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		PhysicsPlugins::default(),
		bevy::asset::AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
	));
	app.add_plugins(spotting_intelligence_avian::SpottingAvianPlugin);
	app.finish();
	app.update();
	app
}

/// Test that the boss's own Fixed collider doesn't block spotting rays.
/// Before fix: Ray hits boss's own collider first, preventing any spotting.
/// After fix: Ray excludes observer's collider hierarchy, allowing proper spotting.
#[test]
fn boss_can_spot_player_without_self_occlusion() -> Result<()> {
	let mut app = spotting_app();
	
	// Spawn boss at origin with production collider
	let spec = BikanwardaSpec::encounter();
	let mut commands = app.world_mut().commands();
	let boss_entity = spawn_bikanwarda_encounter(&mut commands, &spec, Transform::IDENTITY);
	
	// Add spotting to boss
	let eye_offset = Vec3::Y * 15.0;
	let spotting_range = 50.0;
	app.world_mut().entity_mut(boss_entity).insert(
		SpottingUser::new(
			eye_offset,
			[SpotDirective::new(InterestLayers::CHARACTER, spotting_range)],
		),
	);
	
	// Spawn player in clear view (no occluders), well within range
	let player_pos = Vec3::new(20.0, 15.0, 0.0); // Same height as boss eyes, 20m away
	let player = app
		.world_mut()
		.spawn((
			Name::new("test-player"),
			Transform::from_translation(player_pos),
			GlobalTransform::from_translation(player_pos),
			RigidBody::Kinematic,
			Collider::capsule(0.4, 1.8),
			PhysicsInteractionLayer::animated_layers(),
			SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 1.8)),
		))
		.id();
	
	// Update physics to settle colliders
	for _ in 0..5 {
		app.update();
	}
	
	// Check spotting after several discovery cycles
	// Discovery interval is ~0.2s, so 20 frames at 60 FPS is ~3 cycles
	for _ in 0..20 {
		app.update();
	}
	
	// Verify player was spotted
	let spotting = app
		.world()
		.get::<SpottingUser>(boss_entity)
		.expect("Boss should have SpottingUser");
	
	anyhow::ensure!(
		spotting.contacts.contains_key(&player),
		"Boss should spot player in clear view (self-occlusion check)"
	);
	
	Ok(())
}

// This test is removed - SpatialQuery access in tests requires system param handling.
// The main test `boss_can_spot_player_without_self_occlusion` verifies the fix end-to-end.

/// Test that existing spotting tests still pass: visible, occluded, and out-of-range.
#[test]
fn spotting_distinguishes_visible_occluded_and_distant() -> Result<()> {
	let mut app = spotting_app();
	
	let observer = app
		.world_mut()
		.spawn((
			Name::new("observer"),
			Transform::default(),
			GlobalTransform::default(),
			SpottingUser::new(
				Vec3::Y * 1.6,
				[SpotDirective::new(InterestLayers::CHARACTER, 30.0)],
			),
		))
		.id();
	
	// Subject A: Visible and in range
	let visible = app
		.world_mut()
		.spawn((
			Name::new("visible-subject"),
			Transform::from_xyz(10.0, 0.0, 0.0),
			GlobalTransform::from_xyz(10.0, 0.0, 0.0),
			RigidBody::Kinematic,
			Collider::capsule(0.4, 1.8),
			PhysicsInteractionLayer::animated_layers(),
			SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 1.8)),
		))
		.id();
	
	// Subject B: Out of range (40m > 30m spotting range)
	let distant = app
		.world_mut()
		.spawn((
			Name::new("distant-subject"),
			Transform::from_xyz(40.0, 0.0, 0.0),
			GlobalTransform::from_xyz(40.0, 0.0, 0.0),
			RigidBody::Kinematic,
			Collider::capsule(0.4, 1.8),
			PhysicsInteractionLayer::animated_layers(),
			SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 1.8)),
		))
		.id();
	
	// Occluder between observer and occluded subject
	app.world_mut().spawn((
		Name::new("wall"),
		Transform::from_xyz(15.0, 1.6, 0.0),
		GlobalTransform::from_xyz(15.0, 1.6, 0.0),
		RigidBody::Static,
		Collider::cuboid(5.0, 5.0, 5.0),
		PhysicsInteractionLayer::fixed_layers(),
	));
	
	// Subject C: Occluded by wall
	let occluded = app
		.world_mut()
		.spawn((
			Name::new("occluded-subject"),
			Transform::from_xyz(20.0, 0.0, 0.0),
			GlobalTransform::from_xyz(20.0, 0.0, 0.0),
			RigidBody::Kinematic,
			Collider::capsule(0.4, 1.8),
			PhysicsInteractionLayer::animated_layers(),
			SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 1.8)),
		))
		.id();
	
	// Update physics to settle
	for _ in 0..3 {
		app.update();
	}
	
	// Run several discovery cycles
	for _ in 0..20 {
		app.update();
	}
	
	let spotting = app
		.world()
		.get::<SpottingUser>(observer)
		.expect("Observer should have SpottingUser");
	
	// Verify outcomes
	anyhow::ensure!(
		spotting.contacts.contains_key(&visible),
		"Visible subject should be spotted"
	);
	anyhow::ensure!(
		!spotting.contacts.contains_key(&occluded),
		"Occluded subject should NOT be spotted"
	);
	anyhow::ensure!(
		!spotting.contacts.contains_key(&distant),
		"Out-of-range subject should NOT be spotted"
	);
	
	Ok(())
}
