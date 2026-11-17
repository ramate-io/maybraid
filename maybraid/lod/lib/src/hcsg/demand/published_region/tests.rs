use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use bevy::math::Vec3;
use bevy::shape::Aabb3d;

use crate::gen::{Id, OriginalId};
use crate::hcsg::{
	spawn_worker, universal_bounds, GenerationContext, GenerationScheme, HcsgClass, HcsgDemand,
	HcsgStorage, PublishedReadError, PublishedRegion, TicketPoll,
};

fn bounds(x: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 1.0, 1.0, 1.0))
}
fn region() -> Aabb3d {
	Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(256.0, 1.0, 1.0))
}
fn read<T: crate::hcsg::HcsgValue>(
	readout: &mut PublishedRegion<T>,
) -> anyhow::Result<crate::hcsg::PublishedRead> {
	readout.try_read().map_err(|error| anyhow::anyhow!("read: {error:?}"))
}

#[test]
fn available_pages_progress_through_churn_and_refresh_removed_or_replaced_values(
) -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	for x in 0..70 {
		storage.publish(Id::from_cell(bounds(x as f32)), Arc::new(x), bounds(x as f32));
	}
	let mut readout = PublishedRegion::<i32>::new(storage.clone(), demand, region());
	assert!(!read(&mut readout)?.complete);
	assert_eq!(readout.entries().count(), 32);
	for x in 100..102 {
		storage.publish(Id::from_cell(bounds(x as f32)), Arc::new(x), bounds(x as f32));
		assert!(
			!read(&mut readout)?.complete,
			"changed membership cannot certify a completed current read"
		);
	}
	assert_eq!(
		readout.entries().count(),
		70,
		"churn must not reset the cursor before later IDs are read"
	);
	for _ in 0..10 {
		if read(&mut readout)?.complete {
			break;
		}
	}
	assert!(read(&mut readout)?.complete);
	assert_eq!(readout.entries().count(), 72);
	let removed = Id::from_cell(bounds(0.0));
	let replaced = Id::from_cell(bounds(1.0));
	storage.remove::<i32>(removed);
	storage.publish(replaced, Arc::new(900), bounds(1.0));
	for _ in 0..10 {
		if read(&mut readout)?.complete {
			break;
		}
	}
	assert!(read(&mut readout)?.complete);
	assert_eq!(readout.entries().count(), 71);
	assert!(!readout.entries().any(|(id, _)| *id == removed));
	assert_eq!(
		readout
			.entries()
			.find(|(id, _)| **id == replaced)
			.map(|(_, entry)| *entry.value),
		Some(900)
	);
	let count = readout.entries().next().map(|(_, entry)| Arc::strong_count(&entry.value));
	assert!(read(&mut readout)?.complete);
	assert_eq!(
		readout.entries().next().map(|(_, entry)| Arc::strong_count(&entry.value)),
		count,
		"quiet polls clone no stored values"
	);
	Ok(())
}

#[test]
fn epoch_and_resource_identity_prevent_reusing_old_world_snapshots() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	storage.publish(Id::from_cell(bounds(0.0)), Arc::new(42u32), bounds(0.0));
	let mut readout = PublishedRegion::<u32>::new(storage.clone(), demand.clone(), region());
	let old_epoch = read(&mut readout)?.epoch;
	assert_eq!(readout.entries().count(), 1);
	let store = storage.store::<u32>().ok_or_else(|| anyhow::anyhow!("store"))?;
	let guard = store.write();
	assert_eq!(readout.try_read(), Err(PublishedReadError::Busy));
	drop(guard);
	demand.advance_epoch();
	storage.clear::<u32>();
	let current = read(&mut readout)?;
	assert!(current.complete && current.epoch != old_epoch);
	assert!(readout.entries().next().is_none());
	assert!(!readout.is_for(&HcsgStorage::default(), &demand));
	assert!(!readout.is_for(&storage, &HcsgDemand::default()));
	Ok(())
}

struct BlockRoot {
	started: mpsc::SyncSender<()>,
	permit: Mutex<mpsc::Receiver<()>>,
}
crate::seeded_root!(BlockRoot);
struct Blocked;
impl GenerationScheme for Blocked {
	fn original_ids_for(_: &mut GenerationContext, _: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId(Id::from_cell(bounds(0.0)))]
	}
	fn build_with_id(cx: &mut GenerationContext, _: Id) -> Option<(Self, Aabb3d)> {
		let root = cx.get::<BlockRoot>(Id::Universal)?;
		root.started.send(()).ok()?;
		root.permit.lock().ok()?.recv_timeout(Duration::from_secs(5)).ok()?;
		Some((Self, bounds(0.0)))
	}
}

#[test]
fn cached_values_are_readable_while_unrelated_generation_is_blocked() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let (started_tx, started_rx) = mpsc::sync_channel(1);
	let (permit_tx, permit_rx) = mpsc::sync_channel(1);
	storage
		.seed(BlockRoot { started: started_tx, permit: Mutex::new(permit_rx) }, universal_bounds());
	let _worker = spawn_worker(storage.clone(), demand.clone())?;
	let mut request = demand
		.try_request::<Blocked>(region(), HcsgClass::Near)
		.map_err(|error| anyhow::anyhow!("request: {error:?}"))?;
	started_rx.recv_timeout(Duration::from_secs(5))?;
	storage.publish(Id::from_cell(bounds(0.0)), Arc::new(42u32), bounds(0.0));
	let mut readout = PublishedRegion::<u32>::new(storage, demand, region());
	let start = Instant::now();
	assert!(
		read(&mut readout)?.complete,
		"cache pass completeness is independent of worker completion"
	);
	assert_eq!(readout.entries().next().map(|(_, entry)| *entry.value), Some(42));
	assert!(start.elapsed() < Duration::from_secs(1));
	assert_eq!(
		request.try_poll(&readout.storage).map_err(|_| anyhow::anyhow!("busy"))?,
		TicketPoll::Pending
	);
	permit_tx.send(())?;
	Ok(())
}
