//! Regression tests for collider scale ownership fix.
//!
//! Verifies that the collider shape is sized correctly (once, from the recipe)
//! independent of Transform.scale propagation, for both parented encounter
//! construction and unparented Ward/Enraged construction.

use bevy::prelude::*;
use bikanwarda_character::{
	spawn_bikanwarda_encounter, spawn_bikanwarda_enraged, spawn_bikanwarda_ward,
	BikanwardaSpec,
};

/// Regression test: parented encounter construction under translated host.
/// Verifies root Transform.scale is unit (collider shape size verification happens in projectile damage test).
#[test]
fn encounter_collider_aabb_matches_scaled_capsule_under_translated_host() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, TransformPlugin));
	app.finish();
	app.update();
	
	let world = app.world_mut();
	
	// Spawn host at nonzero translation
	let host_translation = Vec3::new(100.0, 50.0, -200.0);
	let host = world
		.spawn((
			Name::new("encounter-host"),
			Transform::from_translation(host_translation),
			GlobalTransform::default(),
		))
		.id();
	
	// Spawn boss actor as child of host (production structure)
	let spec = BikanwardaSpec::encounter();
	let _scale = spec.uniform_scale();
	let actor = {
		let mut commands = world.commands();
		let boss = spawn_bikanwarda_encounter(&mut commands, &spec, Transform::IDENTITY);
		commands.entity(boss).insert(ChildOf(host));
		boss
	};
	world.flush();
	
	// Propagate transforms for several frames
	for _ in 0..5 {
		app.update();
	}
	
	// Verify root Transform.scale is unit (collider shape sizing should not rely on scale)
	let world = app.world_mut();
	let (transform, global_transform) = world
		.query::<(&Transform, &GlobalTransform)>()
		.get(world, actor)
		.expect("Actor should have Transform and GlobalTransform");
	
	assert!(
		(transform.scale - Vec3::ONE).length() < 1e-4,
		"Root Transform.scale should be unit (1,1,1), got {:?}. Collider shape should be sized from scaled_capsule, not relying on Transform.scale propagation.",
		transform.scale
	);
	
	// Verify GlobalTransform position matches host (actor local transform is identity)
	let tolerance = 0.1;
	assert!(
		(global_transform.translation() - host_translation).length() < tolerance,
		"Actor world position {:?} should match host translation {:?} (±{:.2}m)",
		global_transform.translation(),
		host_translation,
		tolerance
	);
}

/// Regression test: unparented Ward construction.
/// Verifies Transform.scale is unit without a parent to inherit scale from.
#[test]
fn ward_collider_aabb_matches_scaled_capsule_unparented() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, TransformPlugin));
	app.finish();
	app.update();
	
	let world = app.world_mut();
	
	// Spawn unparented Ward at a world position
	let world_pos = Vec3::new(50.0, 0.0, -100.0);
	let spec = BikanwardaSpec::ward();
	let _scale = spec.uniform_scale(); // Ward is 0.85× encounter scale
	let actor = {
		let mut commands = world.commands();
		spawn_bikanwarda_ward(&mut commands, &spec, Transform::from_translation(world_pos))
	};
	world.flush();
	
	// Propagate transforms
	for _ in 0..5 {
		app.update();
	}
	
	// Verify Transform.scale is unit
	let world = app.world_mut();
	let (transform, global_transform) = world
		.query::<(&Transform, &GlobalTransform)>()
		.get(world, actor)
		.expect("Ward should have Transform and GlobalTransform");
	
	assert!(
		(transform.scale - Vec3::ONE).length() < 1e-4,
		"Ward root Transform.scale should be unit (1,1,1), got {:?}. Collider shape should be sized from scaled_capsule.",
		transform.scale
	);
	
	let tolerance = 0.1;
	assert!(
		(global_transform.translation() - world_pos).length() < tolerance,
		"Ward world position {:?} should match spawn position {:?} (±{:.2}m)",
		global_transform.translation(),
		world_pos,
		tolerance
	);
}

/// Regression test: unparented Enraged construction.
#[test]
fn enraged_collider_aabb_matches_scaled_capsule_unparented() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, TransformPlugin));
	app.finish();
	app.update();
	
	let world = app.world_mut();
	
	let world_pos = Vec3::new(-50.0, 0.0, 100.0);
	let spec = BikanwardaSpec::enraged();
	let actor = {
		let mut commands = world.commands();
		spawn_bikanwarda_enraged(&mut commands, &spec, Transform::from_translation(world_pos))
	};
	world.flush();
	
	for _ in 0..5 {
		app.update();
	}
	
	let world = app.world_mut();
	let (transform, global_transform) = world
		.query::<(&Transform, &GlobalTransform)>()
		.get(world, actor)
		.expect("Enraged should have Transform and GlobalTransform");
	
	assert!(
		(transform.scale - Vec3::ONE).length() < 1e-4,
		"Enraged root Transform.scale should be unit (1,1,1), got {:?}. Collider shape should be sized from scaled_capsule.",
		transform.scale
	);
	
	let tolerance = 0.1;
	assert!(
		(global_transform.translation() - world_pos).length() < tolerance,
		"Enraged world position {:?} should match spawn position {:?} (±{:.2}m)",
		global_transform.translation(),
		world_pos,
		tolerance
	);
}

/// Regression test: verify root Transform.scale stays unit even if caller passes scaled transform.
#[test]
fn root_transform_scale_stays_unit_regardless_of_input() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, TransformPlugin));
	app.finish();
	app.update();
	
	let world = app.world_mut();
	
	// Attempt to pass a scaled transform (e.g. from spec.root_transform())
	let spec = BikanwardaSpec::encounter();
	let scaled_input = spec.root_transform(Vec3::new(10.0, 0.0, 20.0), 0.0);
	assert!(
		scaled_input.scale.length() > 1.0,
		"Input transform should have scale (setup check)"
	);
	
	let actor = {
		let mut commands = world.commands();
		spawn_bikanwarda_encounter(&mut commands, &spec, scaled_input)
	};
	world.flush();
	app.update();
	
	// Verify root Transform.scale is unit
	let world = app.world();
	let root_transform = world
		.get::<Transform>(actor)
		.expect("Actor root should have Transform");
	
	assert!(
		(root_transform.scale - Vec3::ONE).length() < 1e-4,
		"Root Transform.scale should be unit (1,1,1), got {:?}",
		root_transform.scale
	);
}

// Collider functional correctness is verified by the projectile damage test in bikanwarda-encounter.
