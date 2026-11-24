use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::scene::{Scene, ScenePlugin};
use bevy::shape::Aabb3d;

use super::{Host, Presented, RetiredHost};
use crate::gen::tests::test_utils::stub_scene;
use crate::gen::{Id, OriginalId};
use crate::hcsg::demand::QuantumProgress;
use crate::hcsg::{
	GenerationContext, GenerationScheme, HcsgClass, HcsgDemand, HcsgRegions, HcsgStorage,
	PUBLICATION_PAGE_IDS,
};
use crate::lod_ref::LodRef;
use crate::scene::{LodScene, LodSceneLevel, LodSceneStatus};

struct Pin;
impl GenerationScheme for Pin {
	fn original_ids_for(_: &mut GenerationContext, _: Aabb3d) -> Vec<OriginalId> {
		Vec::new()
	}
	fn build_with_id(_: &mut GenerationContext, _: Id) -> Option<(Self, Aabb3d)> {
		None
	}
}
impl LodScene for Pin {
	fn scene_lod_status(&self, _: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}
	fn scene_with_level(&self, _: &LodRef, _: LodSceneLevel) -> impl Scene + 'static {
		stub_scene()
	}
}

fn bounds(x: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 1.0, 1.0, 1.0))
}
fn state(demand: &HcsgDemand) -> anyhow::Result<Presented<Pin>> {
	let mut state = Presented::default();
	state.request.apply_regions(&HcsgRegions::<()>::new(
		vec![Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(1000.0, 1.0, 1.0))],
		None,
		HcsgClass::Near,
	));
	anyhow::ensure!(state.request(demand), "request should subscribe");
	Ok(state)
}
fn publish(
	storage: &HcsgStorage,
	demand: &HcsgDemand,
	state: &Presented<Pin>,
	count: usize,
	failed: bool,
) -> anyhow::Result<Vec<Id>> {
	let ids: Vec<_> = (0..count)
		.map(|x| {
			let bounds = bounds(x as f32);
			let id = Id::from_cell(bounds);
			storage.publish(id, std::sync::Arc::new(Pin), bounds);
			id
		})
		.collect();
	let subscription =
		state.request.subscription().ok_or_else(|| anyhow::anyhow!("subscription"))?;
	demand.finish_quantum(
		subscription,
		QuantumProgress { published: ids.clone(), done: true, failed, ..Default::default() },
		None,
	);
	Ok(ids)
}
fn drain(
	state: &mut Presented<Pin>,
	storage: &HcsgStorage,
	demand: &HcsgDemand,
	world: &mut World,
) {
	let mut queue = CommandQueue::default();
	state.spawn(storage, demand, &mut Commands::new(&mut queue, world));
	queue.apply(world);
}
fn old_host(
	state: &mut Presented<Pin>,
	world: &mut World,
	storage: &HcsgStorage,
	id: Id,
) -> anyhow::Result<Entity> {
	let entry = storage.blocking_entry::<Pin>(id).ok_or_else(|| anyhow::anyhow!("stored pin"))?;
	let entity = world.spawn_empty().id();
	state.hosts.insert(
		id,
		Host { entity, bounds: entry.bounds, version: entry.version, session: state.session },
	);
	Ok(entity)
}

#[test]
fn new_host_backlog_is_spawned_in_bounded_pages() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let mut state = state(&demand)?;
	publish(&storage, &demand, &state, 70, false)?;
	let mut app = App::new();
	app.add_plugins((AssetPlugin::default(), ScenePlugin));
	for count in [PUBLICATION_PAGE_IDS, PUBLICATION_PAGE_IDS * 2, 70] {
		drain(&mut state, &storage, &demand, app.world_mut());
		assert_eq!(state.hosts.len(), count);
		assert_eq!(state.cursor, count);
	}
	Ok(())
}

#[test]
fn completed_backlog_keeps_unseen_hosts_until_last_page_after_epoch() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let mut state = state(&demand)?;
	let ids = publish(&storage, &demand, &state, 70, false)?;
	let mut world = World::new();
	for &id in &ids {
		old_host(&mut state, &mut world, &storage, id)?;
	}
	let orphan_id = Id::from_cell(bounds(900.0));
	storage.publish(orphan_id, std::sync::Arc::new(Pin), bounds(900.0));
	let orphan = old_host(&mut state, &mut world, &storage, orphan_id)?;
	demand.advance_epoch();
	assert!(state.request(&demand));
	assert_eq!(state.session, 1);
	assert!(state.sweep_pending);
	let subscription = state
		.request
		.subscription()
		.ok_or_else(|| anyhow::anyhow!("new subscription"))?;
	demand.finish_quantum(
		subscription,
		QuantumProgress { published: ids.clone(), done: true, ..Default::default() },
		None,
	);
	for cursor in [PUBLICATION_PAGE_IDS, PUBLICATION_PAGE_IDS * 2] {
		drain(&mut state, &storage, &demand, &mut world);
		assert_eq!(state.cursor, cursor);
		assert_eq!(state.hosts.len(), 71);
		assert!(state.sweep_pending);
		assert!(world.get::<RetiredHost>(orphan).is_none());
		assert_eq!(state.hosts.values().filter(|host| host.session == 1).count(), cursor);
	}
	drain(&mut state, &storage, &demand, &mut world);
	assert_eq!(state.cursor, 70);
	assert!(!state.sweep_pending);
	assert_eq!(state.hosts.len(), 70);
	assert!(world.get::<RetiredHost>(orphan).is_some());
	assert!(ids.iter().all(|id| state
		.hosts
		.get(id)
		.is_some_and(|host| world.get::<RetiredHost>(host.entity).is_none())));
	Ok(())
}

#[test]
fn busy_storage_does_not_consume_publication_or_retire_hosts() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let mut state = state(&demand)?;
	let ids = publish(&storage, &demand, &state, 2, false)?;
	let mut world = World::new();
	for &id in &ids {
		old_host(&mut state, &mut world, &storage, id)?;
	}
	let store = storage.store::<Pin>().ok_or_else(|| anyhow::anyhow!("store"))?;
	let guard = store.write();
	drain(&mut state, &storage, &demand, &mut world);
	assert_eq!(state.cursor, 0);
	assert_eq!(state.hosts.len(), 2);
	drop(guard);
	drain(&mut state, &storage, &demand, &mut world);
	assert_eq!(state.cursor, 2);
	Ok(())
}

#[test]
fn missing_publication_replaces_previous_subscription_and_replays_before_retiring(
) -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let mut state = state(&demand)?;
	let ids = publish(&storage, &demand, &state, 2, false)?;
	let mut world = World::new();
	for &id in &ids {
		old_host(&mut state, &mut world, &storage, id)?;
	}
	state.session += 1;
	state.sweep_pending = true;
	let previous = state
		.request
		.subscription()
		.ok_or_else(|| anyhow::anyhow!("old subscription"))?;
	storage.remove::<Pin>(ids[0]);
	drain(&mut state, &storage, &demand, &mut world);
	assert_eq!(state.cursor, 0, "missing ID is not consumed");
	assert_eq!(
		state.request.subscription(),
		Some(previous),
		"retain predecessor until replacement succeeds"
	);
	assert!(state.sweep_pending);
	assert!(state.hosts.values().all(|host| world.get::<RetiredHost>(host.entity).is_none()));
	assert!(state.request(&demand));
	assert!(demand.try_read(previous, 0).map_err(|_| anyhow::anyhow!("busy"))?.is_none());
	let replacement = state
		.request
		.subscription()
		.ok_or_else(|| anyhow::anyhow!("new subscription"))?;
	assert_ne!(previous, replacement);
	let version = storage.publish(ids[0], std::sync::Arc::new(Pin), bounds(0.0));
	// Keep scene spawning out of this recovery test; this old host now matches
	// the regenerated version and is only marked current by a consumed page.
	state.hosts.get_mut(&ids[0]).ok_or_else(|| anyhow::anyhow!("old host"))?.version = version;
	demand.finish_quantum(
		replacement,
		QuantumProgress { published: ids, done: true, ..Default::default() },
		None,
	);
	drain(&mut state, &storage, &demand, &mut world);
	assert_eq!(state.cursor, 2);
	assert_eq!(state.hosts.len(), 2);
	assert!(!state.sweep_pending);
	Ok(())
}

#[test]
fn failed_completion_never_certifies_unseen_host_retirement() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let mut state = state(&demand)?;
	let ids = publish(&storage, &demand, &state, 1, false)?;
	let mut world = World::new();
	let entity = old_host(&mut state, &mut world, &storage, ids[0])?;
	state.session += 1;
	state.sweep_pending = true;
	let subscription =
		state.request.subscription().ok_or_else(|| anyhow::anyhow!("subscription"))?;
	// The subscription fails after a partial prefix has already been consumed.
	demand.finish_quantum(
		subscription,
		QuantumProgress { done: true, failed: true, ..Default::default() },
		None,
	);
	// Re-reading its completed tail must not make this failure authoritative.
	state.cursor = 1;
	drain(&mut state, &storage, &demand, &mut world);
	assert!(state.sweep_pending);
	assert_eq!(state.hosts.len(), 1);
	assert!(world.get::<RetiredHost>(entity).is_none());
	Ok(())
}
