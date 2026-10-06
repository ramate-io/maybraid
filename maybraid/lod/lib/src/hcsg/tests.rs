use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::gen::tests::test_utils::{
	cell, leaf_id, moss_id, span, tree_id, Leaf, Moss, Terrain, Tree, Vegetation,
};
use crate::gen::{
	Id, LodGenerateBudget, LodGenerated, MaterializeStatus, SpatialIndex, StorageStatus,
};
use crate::hcsg::{GenerateOn, GenerateQueue, GenerationProducer, HcsgStorage, Seed};
use crate::jobs::{ensure_lod_job_counter, LodJobCounter};

#[test]
fn fixture_hierarchy_generates_through_unified_storage() {
	let mut storage = HcsgStorage::default();
	let id = Id::from_cell(cell(0.0));
	assert_eq!(storage.get_or_generate::<Vegetation>(id), Some(MaterializeStatus::Created));
	assert!(storage.contains::<Terrain>(id));
	assert!(storage.contains::<Moss>(moss_id(leaf_id(tree_id(id)))));
	assert_eq!(storage.get_or_generate::<Vegetation>(id), Some(MaterializeStatus::Existing));
}

#[test]
fn types_share_ids_without_colliding() {
	let mut storage = HcsgStorage::default();
	let id = Id::from_cell(cell(3.0));
	storage.insert(id, Terrain { cell: cell(3.0) }, cell(3.0));
	storage.insert(id, Vegetation { cell: cell(3.0) }, cell(3.0));
	assert_eq!(storage.get::<Terrain>(id), Some(&Terrain { cell: cell(3.0) }));
	assert_eq!(storage.get::<Vegetation>(id), Some(&Vegetation { cell: cell(3.0) }));
	storage.remove::<Terrain>(id);
	assert!(storage.contains::<Vegetation>(id));
}

#[test]
fn overlapping_matches_bounds_through_the_spatial_index() {
	let mut storage = HcsgStorage::default();
	for x in 0..200 {
		let bounds = cell(x as f32);
		storage.insert(Id::from_cell(bounds), Terrain { cell: bounds }, bounds);
	}
	let hits = storage.overlapping::<Terrain>(span(10.5, 12.5));
	assert_eq!(
		hits,
		vec![10.0, 11.0, 12.0]
			.into_iter()
			.map(|x| Id::from_cell(cell(x)))
			.collect::<Vec<_>>()
	);
	assert_eq!(storage.overlapping::<Terrain>(span(-1.0e6, 1.0e6)).len(), 200);
}

#[test]
fn versions_never_repeat_across_clears() {
	let mut storage = HcsgStorage::default();
	let id = Id::from_cell(cell(0.0));
	let first = storage.insert(id, Terrain { cell: cell(0.0) }, cell(0.0));
	let revision = SpatialIndex::<Terrain>::membership_revision(&storage);
	storage.clear::<Terrain>();
	assert!(!storage.contains::<Terrain>(id));
	assert!(SpatialIndex::<Terrain>::membership_revision(&storage) > revision);
	let rebuilt = storage.insert(id, Terrain { cell: cell(0.0) }, cell(0.0));
	assert!(rebuilt > first);
}

#[test]
fn clear_group_drops_only_its_members() {
	struct World;
	let mut storage = HcsgStorage::default();
	storage.add_to_group::<World, Terrain>().add_to_group::<World, Tree>();
	let id = Id::from_cell(cell(0.0));
	storage.get_or_generate::<Vegetation>(id);
	storage.clear_group::<World>();
	assert!(!storage.contains::<Terrain>(id));
	assert!(!storage.contains::<Tree>(tree_id(id)));
	assert!(storage.contains::<Vegetation>(id));
	assert!(storage.contains::<Leaf>(leaf_id(tree_id(id))));
}

#[test]
fn relocated_entries_report_tracked_outside() {
	let mut storage = HcsgStorage::default();
	let id = Id::from_cell(cell(0.0));
	let version = storage.insert(id, Terrain { cell: cell(0.0) }, cell(0.0));
	assert!(storage.relocate::<Terrain>(id, cell(50.0)));
	assert_eq!(
		SpatialIndex::<Terrain>::storage_status(&storage, id),
		StorageStatus::TrackedOutside
	);
	assert_eq!(SpatialIndex::<Terrain>::version(&storage, id), Some(version));
	assert_eq!(storage.overlapping::<Terrain>(cell(50.0)), vec![id]);
	assert!(storage.overlapping::<Terrain>(cell(0.0)).is_empty());
}

#[derive(Resource, Clone, Default)]
struct Config(u32);

crate::seeded_root!(Config);

#[test]
fn seeded_roots_are_read_through_get_or_generate() {
	let mut storage = HcsgStorage::default();
	assert!(storage.get_one_or_generate::<Config>(Id::Universal).is_none());
	storage.seed(Config(7), crate::hcsg::universal_bounds());
	assert_eq!(storage.get_one_or_generate::<Config>(Id::Universal).map(|c| c.0), Some(7));
}

struct Window;

#[derive(Resource, Default)]
struct PublishNext(Option<Aabb3d>);

fn publish(mut next: ResMut<PublishNext>, mut channel: GenerationProducer<Window>) {
	if let Some(keep) = next.0.take() {
		channel.publish(keep, Some(Vec2::ZERO));
	}
}

fn app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	ensure_lod_job_counter(&mut app);
	app.init_resource::<PublishNext>()
		.add_plugins((
			GenerateOn::<Window, Terrain>::default(),
			GenerateOn::<Window, Vegetation>::default(),
			Seed::<Config>::default(),
		))
		.insert_resource(LodGenerateBudget::<Window>::new(64))
		.insert_resource(Config(3))
		.add_systems(Update, publish.in_set(crate::gen::LodGenerateSystems::Produce));
	app
}

fn drain_generated<T: Send + Sync + 'static>(app: &mut App) -> usize {
	app.world_mut().resource_mut::<Messages<LodGenerated<T>>>().drain().count()
}

#[test]
fn subscribers_share_one_producer_and_one_cache() {
	let mut app = app();
	app.world_mut().resource_mut::<PublishNext>().0 = Some(span(0.0, 3.5));
	app.update();
	let storage = app.world().resource::<HcsgStorage>();
	for x in 0..4 {
		let id = Id::from_cell(cell(x as f32));
		assert!(storage.contains::<Terrain>(id));
		assert!(storage.contains::<Vegetation>(id));
	}
	assert_eq!(storage.get::<Config>(Id::Universal).map(|c| c.0), Some(3));
	assert_eq!(app.world().resource::<LodJobCounter>().active(), 0);
}

#[test]
fn moving_bounds_scans_only_the_entering_strip() {
	let mut app = app();
	app.world_mut().resource_mut::<PublishNext>().0 = Some(span(0.0, 3.5));
	app.update();
	assert_eq!(drain_generated::<Terrain>(&mut app), 4);
	assert_eq!(drain_generated::<Vegetation>(&mut app), 4);
	app.world_mut().resource_mut::<PublishNext>().0 = Some(span(2.0, 5.5));
	app.update();
	// Cells 3, 4, 5 intersect the entering strip; 3 was stored last frame.
	assert_eq!(drain_generated::<Terrain>(&mut app), 3);
	assert_eq!(drain_generated::<Vegetation>(&mut app), 3);
	assert!(app
		.world()
		.resource::<HcsgStorage>()
		.contains::<Terrain>(Id::from_cell(cell(5.0))));
}

#[test]
fn late_subscriber_reset_rescans_current_bounds() {
	let mut app = app();
	app.world_mut().resource_mut::<PublishNext>().0 = Some(span(0.0, 1.5));
	app.update();
	app.world_mut().resource_mut::<HcsgStorage>().clear::<Vegetation>();
	let cancelled = app.world_mut().resource_mut::<GenerateQueue<Window, Vegetation>>().reset();
	assert_eq!(cancelled, 0);
	app.update();
	let storage = app.world().resource::<HcsgStorage>();
	assert!(storage.contains::<Vegetation>(Id::from_cell(cell(0.0))));
	assert!(storage.contains::<Vegetation>(Id::from_cell(cell(1.0))));
}
