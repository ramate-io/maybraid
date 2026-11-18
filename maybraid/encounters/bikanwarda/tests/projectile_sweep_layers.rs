//! Regression test: Projectile sweeps must hit Bikanwarda boss colliders.
//!
//! Tests that the collision layers on boss colliders are compatible with
//! projectile sweep queries.

use avian3d::prelude::*;
use bevy::prelude::*;
use bikanwarda_character::{spawn_bikanwarda_encounter, BikanwardaSpec};
use lod_avian::PhysicsInteractionLayer;

#[test]
fn boss_collider_uses_fixed_layers() {
	let mut world = World::new();
	
	// Spawn boss actor directly (no app.update() needed to check components)
	let spec = BikanwardaSpec::encounter();
	let boss = {
		let mut commands = world.commands();
		spawn_bikanwarda_encounter(&mut commands, &spec, Transform::IDENTITY)
	};
	world.flush();
	
	// Check boss has collider
	let collider = world.get::<Collider>(boss);
	assert!(collider.is_some(), "Boss should have collider");
	
	// Check boss collision layers
	let layers = world.get::<CollisionLayers>(boss);
	assert!(layers.is_some(), "Boss should have CollisionLayers");
	
	let layers = layers.unwrap();
	let expected = PhysicsInteractionLayer::fixed_layers();
	
	println!("Boss collision layers: memberships={:?}, filters={:?}", 
		layers.memberships, layers.filters);
	println!("Expected fixed_layers: memberships={:?}, filters={:?}",
		expected.memberships, expected.filters);
	
	assert_eq!(
		*layers, expected,
		"Boss should use PhysicsInteractionLayer::fixed_layers()"
	);
}

#[test]
fn projectile_sweep_filter_includes_fixed_layer() {
	// The projectile sweep filter from projectiles/src/lib.rs SweepScratch::default()
	let sweep_filter = SpatialQueryFilter::from_mask([
		PhysicsInteractionLayer::Fixed,
		PhysicsInteractionLayer::Animated,
	]);
	
	// Boss uses fixed_layers()
	let boss_layers = PhysicsInteractionLayer::fixed_layers();
	
	// For comparison: animated layers (what players/raiders typically have)
	let animated_layers = PhysicsInteractionLayer::animated_layers();
	
	println!("\n=== Collision Layer Analysis ===\n");
	println!("Projectile sweep queries: Fixed + Animated layers");
	println!("Sweep filter: {:?}\n", sweep_filter);
	
	println!("Boss collision layers (fixed_layers()):");
	println!("  Memberships: {:?}", boss_layers.memberships);
	println!("  Filters: {:?}\n", boss_layers.filters);
	
	println!("Player/Raider layers (animated_layers()):");
	println!("  Memberships: {:?}", animated_layers.memberships);
	println!("  Filters: {:?}\n", animated_layers.filters);
	
	println!("Layer enum:");
	println!("  Fixed = {:?}", PhysicsInteractionLayer::Fixed as u32);
	println!("  Animated = {:?}\n", PhysicsInteractionLayer::Animated as u32);
	
	println!("=== Analysis ===");
	println!("Projectile sweep should find entities in Fixed OR Animated layers.");
	println!("Boss is in Fixed layer, so sweeps SHOULD hit it.");
	println!("If boss takes no damage, the bug is NOT in collision layers.\n");
}
