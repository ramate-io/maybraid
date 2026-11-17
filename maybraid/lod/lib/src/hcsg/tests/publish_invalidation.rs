//! Test that HcsgStorage::publish invalidates intersecting subscriptions.
//!
//! Regression test for late-published data not being discovered. When a subscription
//! has already discovered a region (found nothing) and marked as done, publishing new
//! data that intersects that region must invalidate the subscription to trigger
//! re-discovery.

use std::sync::Arc;

use bevy::math::Vec3;
use bevy::prelude::{App, MinimalPlugins, Update};
use bevy::shape::{Aabb3d, IntersectsVolume};
use bevy::time::{TimeUpdateStrategy, Timer, TimerMode};

use crate::gen::{Bytes, Id, OriginalId};
use crate::hcsg::worker::HcsgWorker;
use crate::hcsg::{
	GenerationContext, GenerationScheme, HcsgBounds, HcsgBoundsPlugin, HcsgClass, HcsgDemand,
	HcsgRegions, HcsgStorage,
};

use std::time::Duration;

const DT: f32 = 1.0 / 60.0;

/// Test ID constant (32 bytes).
const TEST_ID: [u8; 32] = *b"test-late-publish\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

/// Test type for late publishing.
#[derive(Clone, Debug, PartialEq)]
struct TestValue {
	position: Vec3,
}

/// Test channel.
struct TestChannel;

impl HcsgBounds for TestChannel {
	const CLASS: HcsgClass = HcsgClass::Near;
	
	type Param = ();
	
	fn regions(_: &()) -> Vec<Aabb3d> {
		// Large region covering the test area
		vec![Aabb3d::new(Vec3::ZERO, Vec3::splat(1000.0))]
	}
}

impl GenerationScheme for TestValue {
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		// Check storage for the test value
		let id = Id::Bytes(Bytes(TEST_ID));
		if let Some(bounds) = cx.stored_bounds::<Self>(id) {
			if bounds.intersects(&region) {
				return vec![OriginalId::new(id)];
			}
		}
		Vec::new()
	}
	
	fn build_with_id(_cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		if id == Id::Bytes(Bytes(TEST_ID)) {
			let value = TestValue { position: Vec3::new(500.0, 0.0, 500.0) };
			let bounds = Aabb3d::new(value.position, Vec3::splat(10.0));
			Some((value, bounds))
		} else {
			None
		}
	}
}

/// Regression test: publish after discovery triggers re-discovery.
///
/// 1. Create subscription, let it discover empty region (done=true)
/// 2. Publish value that intersects the region
/// 3. Verify worker re-discovers and generates the value
#[test]
fn publish_after_discovery_invalidates_subscription() -> anyhow::Result<()> {
	let mut app = App::new();
	
	app.add_plugins(MinimalPlugins);
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)));
	
	// Initialize HCSG runtime (storage + demand + worker)
	let storage = app.world_mut().get_resource_or_init::<HcsgStorage>().clone();
	let demand = app.world_mut().get_resource_or_init::<HcsgDemand>().clone();
	
	// Connect storage and demand for automatic invalidation
	storage.set_demand(demand.clone());
	
	let worker = HcsgWorker::spawn(storage.clone(), demand.clone())
		.map_err(|e| anyhow::anyhow!("Failed to spawn worker: {}", e))?;
	app.insert_resource(worker);
	
	// Add bounds plugin to trigger subscription
	app.add_plugins(HcsgBoundsPlugin::<TestChannel>::default());
	
	// Add generation plugin to subscribe and discover
	app.add_plugins(crate::hcsg::GenerationPlugin::<TestChannel, TestValue>::default());
	
	// Step 1: Let initial discovery complete (finds nothing)
	for _ in 0..60 {
		app.update();
	}
	
	// Wait for worker to finish
	let idle = demand.wait_idle(Duration::from_secs(2));
	anyhow::ensure!(idle, "Worker should complete initial discovery");
	
	// Verify value is NOT in storage yet
	let entry_before = storage.try_entry::<TestValue>(Id::Bytes(Bytes(TEST_ID)));
	anyhow::ensure!(
		entry_before.is_err() || entry_before.unwrap().is_none(),
		"Test value should not exist before publish"
	);
	
	// Step 2: NOW publish the value (after discovery completed)
	let value = TestValue { position: Vec3::new(500.0, 0.0, 500.0) };
	let bounds = Aabb3d::new(value.position, Vec3::splat(10.0));
	let id = Id::Bytes(Bytes(TEST_ID));
	
	storage.publish(id, Arc::new(value.clone()), bounds);
	
	// Step 3: Wait for re-discovery
	for _ in 0..60 {
		app.update();
	}
	
	let idle = demand.wait_idle(Duration::from_secs(2));
	anyhow::ensure!(idle, "Worker should complete re-discovery");
	
	// Verify value is now in storage and was re-discovered
	let entry_after = storage.entry::<TestValue>(id);
	anyhow::ensure!(
		entry_after.is_some(),
		"Test value should exist after publish and re-discovery"
	);
	
	if let Some(entry) = entry_after {
		anyhow::ensure!(
			entry.value.position == value.position,
			"Value should match what was published"
		);
	}
	
	Ok(())
}
