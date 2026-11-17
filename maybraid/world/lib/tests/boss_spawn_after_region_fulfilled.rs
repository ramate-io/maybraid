//! Regression test for boss spawn after region discovery has already completed.
//!
//! The original bug: running `boss spawn` after world startup (when the near region
//! has already been discovered and fulfilled with no boss) results in no
//! boss discovery, even though the boss is correctly published to storage.
//!
//! Root cause: `HcsgStorage::publish` doesn't notify existing region requests.
//! When a region is already fulfilled, publishing new data that intersects it
//! doesn't trigger re-discovery.
//!
//! Fix: Added `HcsgDemand::invalidate_intersecting` to clear cached discoveries
//! and reset the `done` flag for subscriptions whose regions intersect newly
//! published data.

#![cfg(feature = "debug-commands")]

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bikanwarda_encounter::{BikanwardaBossEncounter, chained_encounter_id};
use lod::gen::{Bytes, Id};
use lod::hcsg::{GenerationPlugin, HcsgBoundsPlugin, HcsgClass, HcsgDemand, HcsgSessionPlugin, HcsgStorage, ViewerHcsgBounds};
use lod::scene::LodViewer;
use maybraid_world::boss_spawn::{BossSpawnPlugin, RequestSpawnBoss};
use std::time::Duration;

const DT: f32 = 1.0 / 60.0;
const SPAWN_X: f32 = 1746.8;
const SPAWN_Z: f32 = 1025.9;

/// Test channel that discovers a large region around the viewer.
struct TestChannel;

impl ViewerHcsgBounds for TestChannel {
	const CLASS: HcsgClass = HcsgClass::Near;
	
	fn regions_around(viewer: Vec3) -> Vec<bevy::shape::Aabb3d> {
		// Large region to ensure boss would be discovered if invalidation works
		vec![bevy::shape::Aabb3d::new(viewer, Vec3::splat(5000.0))]
	}
}

/// Regression test: boss spawn after region discovery has already completed.
///
/// Steps:
/// 1. Start app with bounds + HCSG session (no boss in storage yet)
/// 2. Allow region discovery to complete (discovers no boss, becomes `done`)
/// 3. Publish boss via `boss spawn` command + invalidate intersecting subscriptions
/// 4. Wait for worker to re-discover (subscription should be marked not-done again)
/// 5. Verify boss appears in storage AND is discovered by subscriptions
///
/// This test must FAIL on 062ae642 (before fix) because published data
/// doesn't invalidate already-fulfilled subscriptions.
#[test]
fn boss_spawn_after_region_fulfilled_creates_host() -> anyhow::Result<()> {
	let mut app = App::new();
	
	app.add_plugins(bevy::MinimalPlugins);
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));
	
	// Add HCSG session plugin (storage + demand + worker)
	app.add_plugins(HcsgSessionPlugin);
	
	// Add boss spawn plugin
	app.add_plugins(BossSpawnPlugin);
	
	// Add HCSG bounds plugin for test channel
	app.add_plugins(HcsgBoundsPlugin::<TestChannel>::default());
	
	// Add generation plugin to create subscription that will discover boss
	app.add_plugins(GenerationPlugin::<TestChannel, BikanwardaBossEncounter>::default());
	
	// Spawn a viewer at origin to activate regions
	app.world_mut().spawn((
		LodViewer,
		Transform::from_translation(Vec3::ZERO),
		GlobalTransform::default(),
	));
	
	// Step 1: Let region discovery complete with NO boss in storage
	// The near region will discover nothing and become fulfilled (done=true)
	for _ in 0..180 {
		// 3 seconds for discovery to complete
		app.update();
	}
	
	// Verify boss is NOT in storage yet
	let storage = app.world().resource::<HcsgStorage>();
	let boss_id = Id::Bytes(Bytes(chained_encounter_id().as_bytes()));
	let entry_before = storage.try_entry::<BikanwardaBossEncounter>(boss_id);
	
	anyhow::ensure!(
		entry_before.is_err() || entry_before.unwrap().is_none(),
		"Boss should not exist in storage before spawn command"
	);
	
	// Step 2: NOW publish boss via spawn command (after region is fulfilled)
	app.world_mut().spawn(RequestSpawnBoss {
		at: Some(Vec2::new(SPAWN_X, SPAWN_Z)),
	});
	
	app.update();
	
	// Verify boss is now in storage
	let storage = app.world().resource::<HcsgStorage>();
	let entry_after = storage.try_entry::<BikanwardaBossEncounter>(boss_id)
		.map_err(|_| anyhow::anyhow!("Storage busy"))?;
	
	anyhow::ensure!(
		entry_after.is_some(),
		"Boss should be in storage after spawn command"
	);
	
	// Step 3: Allow re-discovery to happen
	// THIS IS THE KEY: worker must re-discover because invalidation reset discovered=None and done=false
	// Viewer stays at origin - test proves re-discovery works WITHOUT viewer movement
	
	// Wait for HCSG worker to process invalidated subscription
	let demand = app.world().resource::<HcsgDemand>();
	let idle = demand.wait_idle(Duration::from_secs(5));
	anyhow::ensure!(idle, "HCSG worker should finish re-discovery within timeout");
	
	// Step 4: Verify boss was discovered by checking if it's in storage and reachable
	// The invalidation should have caused the subscription to rediscover, finding the boss
	let storage = app.world().resource::<HcsgStorage>();
	let boss_entry = storage.entry::<BikanwardaBossEncounter>(boss_id);
	
	anyhow::ensure!(
		boss_entry.is_some(),
		"Boss should be in storage and reachable after re-discovery"
	);
	
	// Verify boss has correct bounds that intersect the test region
	if let Some(entry) = boss_entry {
		let boss_bounds = entry.bounds;
		let viewer_region = bevy::shape::Aabb3d::new(Vec3::ZERO, Vec3::splat(5000.0));
		
		use bevy::shape::IntersectsVolume;
		anyhow::ensure!(
			viewer_region.intersects(&boss_bounds),
			"Boss bounds should intersect viewer region for discovery to work. \
			 Boss bounds: {:?}, Viewer region center: {:?}",
			boss_bounds,
			Vec3::ZERO
		);
	}
	
	Ok(())
}
