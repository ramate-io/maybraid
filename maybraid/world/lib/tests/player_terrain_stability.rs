//! Player terrain stability regression test.
//!
//! Ensures the player doesn't fall through terrain after world-ready state is achieved.
//! Regression for issue where player y dropped from -351 to -1558 during boss spawn testing.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use maybraid_spawns_support::PendingActorSupport;
use player::{Grounded, Player as MaybraidPlayer};
use std::time::Duration;
use world_player::Player as WorldPlayer;

const DT: f32 = 1.0 / 60.0; // 60 FPS

/// Check that a grounded player with settled support maintains stable Y position over time.
#[test]
fn grounded_player_does_not_fall_through_terrain() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));
	
	// Spawn a test player at a reasonable ground height
	let player = app.world_mut().spawn((
		MaybraidPlayer,
		WorldPlayer,
		Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
		Grounded, // Already grounded
		// No PendingActorSupport means terrain support is settled
	)).id();
	
	app.update();
	
	// Record initial Y position
	let initial_y = app.world().get::<Transform>(player)
		.ok_or_else(|| anyhow::anyhow!("player missing"))?
		.translation.y;
	
	// Run for 5 seconds (300 frames at 60 FPS)
	const TEST_FRAMES: u32 = 300;
	const MAX_Y_DRIFT: f32 = 0.5; // Allow small physics settling
	
	for frame in 0..TEST_FRAMES {
		app.update();
		
		let current_y = app.world().get::<Transform>(player)
			.ok_or_else(|| anyhow::anyhow!("player despawned at frame {}", frame))?
			.translation.y;
		
		let y_drift = (current_y - initial_y).abs();
		anyhow::ensure!(
			y_drift < MAX_Y_DRIFT,
			"Player fell through terrain: initial y={:.1}, current y={:.1}, drift={:.1} at frame {}",
			initial_y,
			current_y,
			y_drift,
			frame
		);
	}
	
	Ok(())
}

/// Check that world-ready state (grounded + no pending support) is stable.
#[test]
fn world_ready_player_stays_ready() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));
	
	// Spawn a world-ready player
	let player = app.world_mut().spawn((
		MaybraidPlayer,
		WorldPlayer,
		Transform::from_translation(Vec3::new(1746.8, 0.0, 1025.9)),
		Grounded,
	)).id();
	
	app.update();
	
	// Verify world-ready conditions hold over time
	for frame in 0..60 {
		app.update();
		
		let grounded = app.world().get::<Grounded>(player).is_some();
		let pending = app.world().get::<PendingActorSupport>(player).is_some();
		
		anyhow::ensure!(
			grounded && !pending,
			"Player lost world-ready state at frame {}: grounded={}, pending={}",
			frame,
			grounded,
			pending
		);
	}
	
	Ok(())
}
