use std::time::{Duration, Instant};

use bevy::math::Vec3;
use bevy::shape::Aabb3d;

use crate::gen::{Id, OriginalId};
use crate::hcsg::demand::QuantumProgress;
use crate::hcsg::{
	spawn_worker, GenerationContext, GenerationScheme, HcsgClass, HcsgDemand, HcsgStorage,
	TicketPoll, TicketRequestError, MAX_TICKET_REQUESTS,
};

struct Many;

fn bounds(x: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 1.0, 1.0, 1.0))
}

impl GenerationScheme for Many {
	fn original_ids_for(_: &mut GenerationContext, _: Aabb3d) -> Vec<OriginalId> {
		(0..70).map(|i| OriginalId(Id::from_cell(bounds(i as f32)))).collect()
	}
	fn build_with_id(_: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		Some((Self, id.origin_cell_bounds()?))
	}
}

fn request<T: GenerationScheme>(
	demand: &HcsgDemand,
	region: Aabb3d,
) -> anyhow::Result<super::HcsgTicket<T>> {
	demand
		.try_request::<T>(region, HcsgClass::Near)
		.map_err(|error| anyhow::anyhow!("request: {error:?}"))
}

#[test]
fn pending_is_not_empty_and_only_successful_completion_is_ready() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let mut ticket = request::<Many>(&demand, bounds(0.0))?;
	assert_eq!(
		ticket.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?,
		TicketPoll::Pending
	);
	demand.finish_quantum(ticket.id, QuantumProgress { done: true, ..Default::default() }, None);
	assert_eq!(ticket.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?, TicketPoll::Ready);
	assert!(ticket.values().is_empty());
	demand.advance_epoch();
	assert_eq!(ticket.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?, TicketPoll::Stale);
	let mut failed = request::<Many>(&demand, bounds(0.0))?;
	demand.finish_quantum(
		failed.id,
		QuantumProgress { done: true, failed: true, ..Default::default() },
		None,
	);
	assert_eq!(failed.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?, TicketPoll::Failed);
	Ok(())
}

#[test]
fn duplicate_consumers_share_capacity_and_last_release_reclaims_it() -> anyhow::Result<()> {
	let demand = HcsgDemand::default();
	let first = request::<Many>(&demand, bounds(0.0))?;
	let duplicate = request::<Many>(&demand, bounds(0.0))?;
	assert_eq!(first.id, duplicate.id);
	let mut others = Vec::new();
	for i in 1..MAX_TICKET_REQUESTS {
		others.push(request::<Many>(&demand, bounds(i as f32))?);
	}
	assert!(matches!(
		demand.try_request::<Many>(bounds(100.0), HcsgClass::Near),
		Err(TicketRequestError::Capacity)
	));
	drop(first);
	assert!(!duplicate.lease.cancelled.load(std::sync::atomic::Ordering::Acquire));
	assert!(matches!(
		demand.try_request::<Many>(bounds(100.0), HcsgClass::Near),
		Err(TicketRequestError::Capacity)
	));
	drop(duplicate);
	let replacement = request::<Many>(&demand, bounds(100.0))?;
	assert_eq!(demand.lock().subscriptions.len(), MAX_TICKET_REQUESTS);
	drop(replacement);
	drop(others);
	Ok(())
}

#[test]
fn bounded_poll_keeps_snapshots_and_missing_publications_are_stale() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let _worker = spawn_worker(storage.clone(), demand.clone())?;
	let region = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(70.0, 1.0, 1.0));
	let mut ticket = request::<Many>(&demand, region)?;
	assert!(demand.wait_idle(Duration::from_secs(5)));
	assert_eq!(
		ticket.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?,
		TicketPoll::Pending
	);
	assert_eq!(ticket.values().len(), 32);
	assert_eq!(
		ticket.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?,
		TicketPoll::Pending
	);
	assert_eq!(ticket.values().len(), 64);
	assert_eq!(ticket.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?, TicketPoll::Ready);
	assert_eq!(ticket.values().len(), 70);
	assert_eq!(ticket.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?, TicketPoll::Ready);
	assert_eq!(ticket.values().len(), 70);
	let mut duplicate = request::<Many>(&demand, region)?;
	storage.clear::<Many>();
	assert_eq!(
		duplicate.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?,
		TicketPoll::Stale
	);
	Ok(())
}

#[test]
fn last_release_wakes_idle_worker_and_evicts_retained_values() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let _worker = spawn_worker(storage.clone(), demand.clone())?;
	let region = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(70.0, 1.0, 1.0));
	let ticket = request::<Many>(&demand, region)?;
	assert!(demand.wait_idle(Duration::from_secs(5)));
	assert!(storage.contains::<Many>(Id::from_cell(bounds(0.0))));
	drop(ticket);
	let deadline = Instant::now() + Duration::from_secs(5);
	while storage.contains::<Many>(Id::from_cell(bounds(0.0))) && Instant::now() < deadline {
		std::thread::sleep(Duration::from_millis(5));
	}
	assert!(!storage.contains::<Many>(Id::from_cell(bounds(0.0))));
	assert!(demand.lock().subscriptions.is_empty());
	Ok(())
}

#[test]
fn invalid_regions_are_distinct_from_transient_busy() {
	let demand = HcsgDemand::default();
	let invalid = Aabb3d { min: Vec3::splat(f32::NAN).into(), max: Vec3::ONE.into() };
	assert!(matches!(
		demand.try_request::<Many>(invalid, HcsgClass::Near),
		Err(TicketRequestError::InvalidRegion)
	));
}

#[test]
fn epoch_cancellation_wins_after_a_busy_storage_snapshot() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let id = Id::from_cell(bounds(0.0));
	storage.publish(id, std::sync::Arc::new(Many), bounds(0.0));
	let mut ticket = request::<Many>(&demand, bounds(0.0))?;
	demand.finish_quantum(
		ticket.id,
		QuantumProgress { published: vec![id], done: true, ..Default::default() },
		None,
	);
	let store = storage.store::<Many>().ok_or_else(|| anyhow::anyhow!("published store"))?;
	let guard = store.write();
	assert!(ticket.try_poll(&storage).is_err());
	assert!(ticket.values().is_empty());
	demand.advance_epoch();
	drop(guard);
	assert_eq!(ticket.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?, TicketPoll::Stale);
	assert!(ticket.values().is_empty());
	Ok(())
}

struct Panicking;
impl GenerationScheme for Panicking {
	fn original_ids_for(_: &mut GenerationContext, _: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId(Id::from_cell(bounds(0.0)))]
	}
	fn build_with_id(_: &mut GenerationContext, _: Id) -> Option<(Self, Aabb3d)> {
		// Simulate a dependency unwinding through generation; the worker catches
		// it and must never certify the resulting partial subscription as empty.
		std::panic::resume_unwind(Box::new("simulated scheme failure"))
	}
}

#[test]
fn caught_worker_panic_is_failed_rather_than_ready_empty() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let _worker = spawn_worker(storage.clone(), demand.clone())?;
	let mut ticket = request::<Panicking>(&demand, bounds(0.0))?;
	assert!(demand.wait_idle(Duration::from_secs(5)));
	assert_eq!(ticket.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?, TicketPoll::Failed);
	assert!(ticket.values().is_empty());
	Ok(())
}

struct BlockingRoot {
	started: std::sync::mpsc::SyncSender<()>,
	permit: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}
crate::seeded_root!(BlockingRoot);
struct Blocking;
impl GenerationScheme for Blocking {
	fn original_ids_for(_: &mut GenerationContext, _: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId(Id::from_cell(bounds(0.0)))]
	}
	fn build_with_id(cx: &mut GenerationContext, _: Id) -> Option<(Self, Aabb3d)> {
		let root = cx.get::<BlockingRoot>(Id::Universal)?;
		root.started.send(()).ok()?;
		root.permit.lock().ok()?.recv_timeout(Duration::from_secs(5)).ok()?;
		Some((Self, bounds(0.0)))
	}
}

#[test]
fn frame_can_poll_and_cancel_while_worker_is_inside_a_dependency() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let (started_tx, started_rx) = std::sync::mpsc::sync_channel(1);
	let (permit_tx, permit_rx) = std::sync::mpsc::sync_channel(1);
	storage.seed(
		BlockingRoot { started: started_tx, permit: std::sync::Mutex::new(permit_rx) },
		crate::hcsg::universal_bounds(),
	);
	let _worker = spawn_worker(storage.clone(), demand.clone())?;
	let mut ticket = request::<Blocking>(&demand, bounds(0.0))?;
	started_rx.recv_timeout(Duration::from_secs(5))?;
	let frame_started = Instant::now();
	assert_eq!(
		ticket.try_poll(&storage).map_err(|_| anyhow::anyhow!("busy"))?,
		TicketPoll::Pending
	);
	let id = ticket.id;
	drop(ticket);
	assert!(
		frame_started.elapsed() < Duration::from_secs(1),
		"frame polling/release waited for the blocked worker"
	);
	assert!(demand.lock().subscriptions.get(&id).is_some_and(|subscription| subscription
		.cancelled
		.load(std::sync::atomic::Ordering::Acquire)));
	permit_tx.send(())?;
	assert!(demand.wait_idle(Duration::from_secs(5)));
	assert!(!storage.contains::<Blocking>(Id::from_cell(bounds(0.0))));
	Ok(())
}
