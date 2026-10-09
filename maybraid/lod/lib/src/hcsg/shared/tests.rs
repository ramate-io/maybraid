use std::sync::Arc;
use std::time::Duration;

use bevy::math::bounding::Aabb3d;
use bevy::math::{DVec3, Vec3};

use crate::gen::tests::test_utils::{
	cell, leaf_id, moss_id, tree_id, Leaf, Moss, Terrain, Tree, Vegetation,
};
use crate::gen::{GeneratingSpatialIndex, Id, OriginalId, SpatialIndex};

use super::{GenerationContext, GenerationScheme, HcsgDemand, HcsgStorage, HcsgWorker};

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
		demand.subscribe::<Cover>(None, span(0.0, 4.0), Some(Vec3::new(3.5, 0.5, 0.5)));
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

	let subscription = demand.subscribe::<Ground>(None, span(0.0, 1.0), None);
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

	let first = demand.subscribe::<Ground>(None, span(0.0, 2.0), None);
	let second = demand.subscribe::<Ground>(Some(first), span(10.0, 2.0), None);
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
	let ground = demand.subscribe::<Ground>(None, span(0.0, 2.0), None);
	let cover = demand.subscribe::<Cover>(None, span(0.0, 2.0), None);
	assert_eq!(demand.advance_epoch(), 1);
	assert_eq!(demand.epoch(), 1);
	assert_eq!(demand.try_read_published(ground, 0), Ok(None));
	assert_eq!(demand.try_read_published(cover, 0), Ok(None));
	assert!(demand.wait_idle(Duration::ZERO));
}

#[test]
fn legacy_schemes_generate_through_the_context() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(0.0));
	let vegetation = GenerationContext::new(&storage).get_or_generate::<Vegetation>(id);
	assert_eq!(vegetation.as_deref(), Some(&Vegetation { cell: cell(0.0) }));
	assert!(storage.contains::<Terrain>(id));
	assert!(!storage.contains::<Tree>(tree_id(id)), "the shared runtime runs no descendants");
}

#[test]
fn legacy_entry_points_still_run_descendants() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(0.0));
	let mut cx = GenerationContext::new(&storage);
	assert!(GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut cx, id).is_some());
	assert!(storage.contains::<Leaf>(leaf_id(tree_id(id))));
	assert!(storage.contains::<Moss>(moss_id(leaf_id(tree_id(id)))));
}

#[test]
fn legacy_reads_outlive_removal() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(4.0));
	storage.publish(id, Arc::new(Terrain { cell: cell(4.0) }), cell(4.0));
	let cx = GenerationContext::new(&storage);
	let terrain = SpatialIndex::<Terrain>::get(&cx, id);
	storage.remove::<Terrain>(id);
	assert_eq!(terrain, Some(&Terrain { cell: cell(4.0) }));
	assert!(SpatialIndex::<Terrain>::get_bounds(&cx, id).is_none());
}

#[test]
fn stale_legacy_inserts_publish_nothing() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(0.0));
	let stale = || true;
	let mut cx = GenerationContext::with_stale(&storage, &stale);
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut cx, id);
	assert!(!storage.contains::<Terrain>(id));
	assert!(!storage.contains::<Vegetation>(id));
}

#[test]
fn worker_fills_legacy_subscriptions() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;

	let subscription = demand.subscribe::<Vegetation>(None, span(0.2, 2.6), None);
	assert!(demand.wait_idle(IDLE));
	let published = demand.try_read_published(subscription, 0).ok().flatten().unwrap_or_default();
	let expected: Vec<Id> = [0.0, 1.0, 2.0].map(|x| Id::from_cell(cell(x))).into();
	assert_eq!(published, expected);
	assert!(published.iter().all(|&id| storage.contains::<Terrain>(id)));
	Ok(())
}

#[test]
fn a_panicking_scheme_does_not_stop_the_worker() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;

	let panicky = demand.subscribe::<Panicky>(None, span(0.0, 1.0), None);
	assert!(demand.wait_idle(IDLE));
	assert_eq!(demand.try_read_published(panicky, 0), Ok(Some(Vec::new())));

	let ground = demand.subscribe::<Ground>(None, span(0.0, 1.0), None);
	assert!(demand.wait_idle(IDLE));
	assert_eq!(demand.try_read_published(ground, 0).ok().flatten().map(|ids| ids.len()), Some(1));
	Ok(())
}
