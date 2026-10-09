use std::sync::Arc;
use std::time::Duration;

use bevy::math::bounding::Aabb3d;
use bevy::math::{DVec3, Vec3};

use crate::gen::tests::test_utils::cell;
use crate::gen::{Id, OriginalId};

use super::bounds::HcsgClass;
use super::demand::{QuantumProgress, SubscriptionId, QUANTUM_IDS};
use super::{GenerationContext, GenerationScheme, HcsgDemand, HcsgStorage, HcsgWorker};

fn subscribe<T: GenerationScheme>(
	demand: &HcsgDemand,
	previous: Option<SubscriptionId>,
	regions: Vec<Aabb3d>,
	focus: Option<Vec3>,
) -> SubscriptionId {
	demand.subscribe::<T>(previous, regions, focus, HcsgClass::Near)
}

fn subscribe_class<T: GenerationScheme>(
	demand: &HcsgDemand,
	previous: Option<SubscriptionId>,
	regions: Vec<Aabb3d>,
	class: HcsgClass,
) -> SubscriptionId {
	demand.subscribe::<T>(previous, regions, None, class)
}

fn yield_quantum(
	demand: &HcsgDemand,
	id: SubscriptionId,
	discovered: Option<Vec<Id>>,
	cursor: usize,
	cost: u32,
	done: bool,
) {
	demand.finish_quantum(id, QuantumProgress { discovered, cursor, cost, done });
}

const IDLE: Duration = Duration::from_secs(10);

/// Session root every [`Ground`] reads.
struct Root {
	seed: u32,
}

#[derive(Debug, PartialEq)]
struct Ground {
	cell: Aabb3d,
	seed: u32,
}

/// Depends on the [`Ground`] at the same id.
struct Cover {
	ground: Arc<Ground>,
}

/// Depends on itself.
struct Cycle;

struct Panicky;

fn span(x: f32, width: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + width, 1.0, 1.0))
}

fn cells_in(region: Aabb3d) -> Vec<OriginalId> {
	let start = region.min.x.floor() as i32;
	let end = region.max.x.ceil() as i32;
	(start..end).map(|x| OriginalId::new(Id::from_cell(cell(x as f32)))).collect()
}

impl GenerationScheme for Ground {
	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cells_in(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let root = cx.get::<Root>(Id::Universal)?;
		let cell = id.origin_cell_bounds()?;
		Some((Self { cell, seed: root.seed }, cell))
	}
}

impl GenerationScheme for Cover {
	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cells_in(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let ground = cx.get_or_generate::<Ground>(id)?;
		let bounds = ground.cell;
		Some((Self { ground }, bounds))
	}
}

impl GenerationScheme for Cycle {
	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cells_in(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		cx.get_or_generate::<Cycle>(id)?;
		Some((Self, cell(0.0)))
	}
}

impl GenerationScheme for Panicky {
	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cells_in(region)
	}

	fn build_with_id(_: &mut GenerationContext, _: Id) -> Option<(Self, Aabb3d)> {
		panic!("scheme failure");
	}
}

fn seeded() -> HcsgStorage {
	let storage = HcsgStorage::default();
	storage.seed(Root { seed: 7 }, span(-1_000.0, 2_000.0));
	storage
}

#[test]
fn publish_shares_values_and_bumps_versions() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(2.0));
	let first = storage.publish(id, Arc::new(Ground { cell: cell(2.0), seed: 1 }), cell(2.0));
	let value = Arc::new(Ground { cell: cell(2.0), seed: 2 });
	let second = storage.publish(id, Arc::clone(&value), cell(2.0));
	assert!(second > first);
	assert!(storage.get::<Ground>(id).is_some_and(|stored| Arc::ptr_eq(&stored, &value)));
	assert_eq!(storage.try_entry::<Ground>(id).ok().flatten().map(|e| e.version), Some(second));
	assert_eq!(storage.overlapping::<Ground>(span(0.0, 5.0)), vec![id]);
	assert!(storage.overlapping::<Cover>(span(0.0, 5.0)).is_empty());

	let revision = storage.membership_revision::<Ground>();
	assert!(storage.remove::<Ground>(id).is_some());
	assert!(storage.membership_revision::<Ground>() > revision);
	assert!(storage
		.try_overlapping::<Ground>(span(0.0, 5.0))
		.is_ok_and(|ids| ids.is_empty()));
}

#[test]
fn configure_rebuilds_the_index_without_losing_values() {
	let storage = HcsgStorage::default();
	for x in 0..50 {
		let id = Id::from_cell(cell(x as f32));
		storage.publish(id, Arc::new(Ground { cell: cell(x as f32), seed: 0 }), cell(x as f32));
	}
	storage.configure::<Ground>(DVec3::splat(4.0));
	assert_eq!(storage.overlapping::<Ground>(span(10.2, 2.0)).len(), 3);
	storage.clear::<Ground>();
	assert!(storage.overlapping::<Ground>(span(0.0, 50.0)).is_empty());
}

#[test]
fn context_generates_dependencies_once() {
	let storage = seeded();
	let id = Id::from_cell(cell(1.0));
	let mut cx = GenerationContext::new(&storage);
	let cover = cx.get_or_generate::<Cover>(id);
	assert_eq!(cover.as_ref().map(|cover| cover.ground.seed), Some(7));
	assert!(storage.contains::<Ground>(id));

	let again = cx.get_or_generate::<Cover>(id);
	assert!(matches!((cover, again), (Some(a), Some(b)) if Arc::ptr_eq(&a, &b)));
}

#[test]
fn context_returns_none_without_roots() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(1.0));
	assert!(GenerationContext::new(&storage).get_or_generate::<Cover>(id).is_none());
	assert!(!storage.contains::<Ground>(id));
}

#[test]
fn context_breaks_cycles() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(0.0));
	assert!(GenerationContext::new(&storage).get_or_generate::<Cycle>(id).is_none());
	assert!(!storage.contains::<Cycle>(id));
}

#[test]
fn stale_context_publishes_nothing() {
	let storage = seeded();
	let id = Id::from_cell(cell(0.0));
	let stale = || true;
	let mut cx = GenerationContext::with_stale(&storage, &stale);
	assert!(cx.get_or_generate::<Cover>(id).is_none());
	assert!(!storage.contains::<Ground>(id));
}

#[test]
fn a_value_that_goes_stale_mid_build_is_not_published() {
	let storage = seeded();
	let id = Id::from_cell(cell(0.0));
	let checks = std::sync::atomic::AtomicUsize::new(0);
	let stale = || checks.fetch_add(1, std::sync::atomic::Ordering::Relaxed) > 0;
	let mut cx = GenerationContext::with_stale(&storage, &stale);
	assert!(cx.get_or_generate::<Ground>(id).is_none());
	assert!(!storage.contains::<Ground>(id));
}

#[test]
fn worker_fills_subscription_nearest_first() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;

	let subscription =
		subscribe::<Cover>(&demand, None, vec![span(0.0, 4.0)], Some(Vec3::new(3.5, 0.5, 0.5)));
	assert!(demand.wait_idle(IDLE));

	let published = demand.try_read_published(subscription, 0).ok().flatten().unwrap_or_default();
	let expected: Vec<Id> = [3.0, 2.0, 1.0, 0.0].map(|x| Id::from_cell(cell(x))).into();
	assert_eq!(published, expected);
	assert!(published
		.iter()
		.all(|&id| storage.contains::<Cover>(id) && storage.contains::<Ground>(id)));

	let tail = demand.try_read_published(subscription, 2).ok().flatten();
	assert_eq!(tail, Some(expected[2..].to_vec()));
	assert_eq!(demand.try_read_published(subscription, 9).ok().flatten(), Some(Vec::new()));
	Ok(())
}

#[test]
fn worker_republishes_existing_values() -> anyhow::Result<()> {
	let storage = seeded();
	let id = Id::from_cell(cell(0.0));
	let existing = GenerationContext::new(&storage).get_or_generate::<Ground>(id);
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;

	let subscription = subscribe::<Ground>(&demand, None, vec![span(0.0, 1.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert_eq!(demand.try_read_published(subscription, 0).ok().flatten(), Some(vec![id]));
	assert!(
		matches!((existing, storage.get::<Ground>(id)), (Some(a), Some(b)) if Arc::ptr_eq(&a, &b))
	);
	Ok(())
}

#[test]
fn replacing_a_subscription_drops_the_previous() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;

	let first = subscribe::<Ground>(&demand, None, vec![span(0.0, 2.0)], None);
	let second = subscribe::<Ground>(&demand, Some(first), vec![span(10.0, 2.0)], None);
	assert_eq!(demand.try_read_published(first, 0), Ok(None));
	assert!(demand.wait_idle(IDLE));
	assert_eq!(demand.try_read_published(second, 0).ok().flatten().map(|ids| ids.len()), Some(2));

	demand.unsubscribe(second);
	assert_eq!(demand.try_read_published(second, 0), Ok(None));
	Ok(())
}

#[test]
fn advancing_the_epoch_drops_every_subscription() {
	let demand = HcsgDemand::default();
	let ground = subscribe::<Ground>(&demand, None, vec![span(0.0, 2.0)], None);
	let cover = subscribe::<Cover>(&demand, None, vec![span(0.0, 2.0)], None);
	assert_eq!(demand.advance_epoch(), 1);
	assert_eq!(demand.epoch(), 1);
	assert_eq!(demand.try_read_published(ground, 0), Ok(None));
	assert_eq!(demand.try_read_published(cover, 0), Ok(None));
	assert!(demand.wait_idle(Duration::ZERO));
}

#[test]
fn region_pulls_skip_what_does_not_exist() {
	let storage = HcsgStorage::default();
	storage.seed(Root { seed: 3 }, span(-1_000.0, 2_000.0));
	let mut cx = GenerationContext::new(&storage);
	let grounds = cx.get_or_generate_in::<Ground>(span(0.2, 2.6));
	let cells: Vec<Aabb3d> = grounds.iter().map(|ground| ground.cell).collect();
	assert_eq!(cells, vec![cell(0.0), cell(1.0), cell(2.0)]);
	assert!(cx.get_or_generate_in::<Cycle>(span(0.0, 2.0)).is_empty());
}

fn cover_seeds(storage: &HcsgStorage) -> std::collections::BTreeMap<Id, u32> {
	storage
		.overlapping::<Cover>(span(-1.0, 20.0))
		.into_iter()
		.filter_map(|id| {
			let cover = storage.get::<Cover>(id)?;
			Some((id, cover.ground.seed))
		})
		.collect()
}

fn worker_cover_state(regions: Vec<Aabb3d>) -> anyhow::Result<std::collections::BTreeMap<Id, u32>> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;
	let subscription = subscribe::<Cover>(&demand, None, regions, None);
	anyhow::ensure!(demand.wait_idle(IDLE));
	anyhow::ensure!(
		demand.try_read_published(subscription, 0).ok().flatten().is_some(),
		"subscription finished"
	);
	Ok(cover_seeds(&storage))
}

#[test]
fn generation_results_do_not_depend_on_demand_order() -> anyhow::Result<()> {
	let one_region = worker_cover_state(vec![span(0.0, 4.0)])?;
	let split_regions =
		worker_cover_state(vec![span(2.0, 1.0), span(0.0, 1.0), span(1.0, 1.0), span(3.0, 1.0)])?;
	anyhow::ensure!(one_region == split_regions, "region order changed stored values");
	Ok(())
}

#[test]
fn a_panicking_scheme_does_not_stop_the_worker() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;

	let panicky = subscribe::<Panicky>(&demand, None, vec![span(0.0, 1.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert_eq!(demand.try_read_published(panicky, 0), Ok(Some(Vec::new())));

	let ground = subscribe::<Ground>(&demand, None, vec![span(0.0, 1.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert_eq!(demand.try_read_published(ground, 0).ok().flatten().map(|ids| ids.len()), Some(1));
	Ok(())
}

#[test]
fn stride_schedule_is_fair_by_class_weight() {
	let demand = HcsgDemand::default();
	let near = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Near);
	let ambient = subscribe_class::<Cover>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Ambient);
	let mut near_quanta = 0u32;
	let mut ambient_quanta = 0u32;
	for _ in 0..36 {
		let job = demand.try_pick().expect("unfinished work");
		if job.id == near {
			near_quanta += 1;
		} else {
			assert_eq!(job.id, ambient);
			ambient_quanta += 1;
		}
		yield_quantum(&demand, job.id, None, 0, QUANTUM_IDS as u32, false);
	}
	assert_eq!((near_quanta, ambient_quanta), (32, 4), "weight 8 and weight 1 run 8:1");
}

#[test]
fn a_weight_1_subscription_completes_while_higher_classes_resubscribe() {
	let demand = HcsgDemand::default();
	let ambient = subscribe_class::<Cover>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Ambient);
	let mut near = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Near);
	let mut ambient_cost = 0u32;
	for _ in 0..64 {
		let job = demand.try_pick().expect("unfinished work");
		if job.id == ambient {
			ambient_cost += QUANTUM_IDS as u32;
			let done = ambient_cost >= 3 * QUANTUM_IDS as u32;
			yield_quantum(&demand, job.id, None, 0, QUANTUM_IDS as u32, done);
			if done {
				assert!(demand.try_read(ambient, 0).unwrap().unwrap().done);
				return;
			}
		} else {
			yield_quantum(&demand, job.id, None, 0, QUANTUM_IDS as u32, false);
			near = subscribe_class::<Ground>(
				&demand,
				Some(near),
				vec![span(0.0, 1.0)],
				HcsgClass::Near,
			);
		}
	}
	panic!("ambient work starved");
}

#[test]
fn replacing_a_subscription_inherits_pass() {
	let demand = HcsgDemand::default();
	let ambient = subscribe_class::<Cover>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Ambient);
	let near = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Near);
	let first = demand.try_pick().expect("near is newer at the same pass");
	assert_eq!(first.id, near);
	yield_quantum(&demand, first.id, None, 0, QUANTUM_IDS as u32, false);
	let inherited = demand.pass_of(near).expect("near still live");
	assert!(inherited > 0.0);
	let replaced =
		subscribe_class::<Ground>(&demand, Some(near), vec![span(1.0, 1.0)], HcsgClass::Near);
	assert_eq!(demand.pass_of(replaced), Some(inherited));
	let next = demand.try_pick().expect("ambient is behind");
	assert_eq!(next.id, ambient, "resubscribing must not jump the queue");
	yield_quantum(&demand, next.id, None, 0, 1, true);
}

#[test]
fn a_switched_out_subscription_resumes_without_rediscovering() {
	let demand = HcsgDemand::default();
	let first = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 8.0)], HcsgClass::Ambient);
	let job = demand.try_pick().expect("only work");
	assert!(job.discovered.is_none());
	let ids: Vec<Id> = (0..8).map(|x| Id::from_cell(cell(x as f32))).collect();
	yield_quantum(&demand, job.id, Some(ids.clone()), 3, 3, false);
	let _other = subscribe_class::<Cover>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Near);
	let other = demand.try_pick().expect("near work");
	assert_ne!(other.id, first);
	yield_quantum(&demand, other.id, None, 0, 1, true);
	let resume = demand.try_pick().expect("first still unfinished");
	assert_eq!(resume.id, first);
	assert_eq!(resume.cursor, 3);
	assert_eq!(resume.discovered.as_deref(), Some(ids.as_slice()));
	yield_quantum(&demand, resume.id, resume.discovered, resume.cursor, 1, true);
}

#[test]
fn a_lone_subscription_fills_one_quantum_at_a_time() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;
	let n = 100u32;
	let subscription = subscribe::<Ground>(&demand, None, vec![span(0.0, n as f32)], None);
	assert!(demand.wait_idle(IDLE));
	let published = demand.try_read_published(subscription, 0).ok().flatten().unwrap_or_default();
	let expected: Vec<Id> = (0..n).map(|x| Id::from_cell(cell(x as f32))).collect();
	assert_eq!(published, expected);
	let picks = demand.scheduler_picks();
	let quanta = u64::from(n.div_ceil(QUANTUM_IDS as u32));
	assert!(picks >= quanta, "a lone subscription must yield each quantum, got {picks} picks");
	assert!(picks < u64::from(n), "must not return to the scheduler once per id, got {picks}");
	Ok(())
}
