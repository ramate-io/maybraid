use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::gen::tests::test_utils::{
	cell, leaf_id, moss_id, span, tree_id, Leaf, Moss, Terrain, Tree, Vegetation,
};
use crate::gen::{
	GeneratingSpatialIndex, GenerationScheme, Id, LodGenerateBudget, LodGenerated,
	MaterializeStatus, OriginalId, SpatialIndex, StorageStatus,
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

#[derive(Resource, Default)]
struct RestartNext(Option<Aabb3d>);

fn publish(
	mut next: ResMut<PublishNext>,
	mut restart: ResMut<RestartNext>,
	mut channel: GenerationProducer<Window>,
) {
	if let Some(keep) = next.0.take() {
		channel.publish(keep, Some(Vec2::ZERO));
	}
	if let Some(keep) = restart.0.take() {
		channel.restart(keep, Some(Vec2::ZERO));
	}
}

fn app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	ensure_lod_job_counter(&mut app);
	app.init_resource::<PublishNext>()
		.init_resource::<RestartNext>()
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

#[test]
fn stationary_window_regenerates_cleared_records_only_after_restart() {
	let mut app = app();
	let keep = span(0.0, 1.5);
	app.world_mut().resource_mut::<PublishNext>().0 = Some(keep);
	app.update();
	let id = Id::from_cell(cell(0.0));
	assert!(app.world().resource::<HcsgStorage>().contains::<Terrain>(id));

	app.world_mut().resource_mut::<HcsgStorage>().clear::<Terrain>();
	app.update();
	assert!(
		!app.world().resource::<HcsgStorage>().contains::<Terrain>(id),
		"clearing records alone does not wake subscribers on an unmoved window"
	);

	app.world_mut().resource_mut::<RestartNext>().0 = Some(keep);
	app.update();
	let storage = app.world().resource::<HcsgStorage>();
	assert!(storage.contains::<Terrain>(id));
	assert!(storage.contains::<Terrain>(Id::from_cell(cell(1.0))));
	assert_eq!(app.world().resource::<LodJobCounter>().active(), 0);
}

#[derive(Resource, Clone, PartialEq, Default)]
struct Shape(u32);

crate::seeded_root!(Shape);

/// Records derived from [`Shape`]: invalidated when it reseeds.
struct ShapedNodes;

#[derive(Debug, PartialEq)]
struct ShapedCell(u32);

impl<S> GenerationScheme<S> for ShapedCell
where
	S: GeneratingSpatialIndex<Terrain> + GeneratingSpatialIndex<Shape>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		GeneratingSpatialIndex::<Terrain>::original_ids_for(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id) -> Option<(Self, Aabb3d)> {
		let shape =
			GeneratingSpatialIndex::<Shape>::get_one_or_generate(spatial_index, Id::Universal)?;
		Some((Self(shape.0), id.origin_cell_bounds()?))
	}
}

fn shaped_app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	ensure_lod_job_counter(&mut app);
	app.world_mut().init_resource::<HcsgStorage>();
	app.world_mut()
		.resource_mut::<HcsgStorage>()
		.add_to_group::<ShapedNodes, ShapedCell>();
	app.init_resource::<PublishNext>()
		.init_resource::<RestartNext>()
		.insert_resource(Shape(1))
		.add_plugins((
			GenerateOn::<Window, ShapedCell>::default(),
			Seed::<Shape>::default().invalidates::<ShapedNodes>().restarts::<Window>(),
		))
		.insert_resource(LodGenerateBudget::<Window>::new(64))
		.add_systems(Update, publish.in_set(crate::gen::LodGenerateSystems::Produce));
	app
}

#[test]
fn reseeding_a_changed_root_regenerates_a_stationary_window() {
	let mut app = shaped_app();
	let id = Id::from_cell(cell(0.0));
	app.world_mut().resource_mut::<PublishNext>().0 = Some(span(0.0, 1.5));
	app.update();
	let first = app.world().resource::<HcsgStorage>().entry::<ShapedCell>(id).map(|e| e.version);
	assert_eq!(app.world().resource::<HcsgStorage>().get::<ShapedCell>(id), Some(&ShapedCell(1)));

	app.world_mut().resource_mut::<Shape>().0 = 2;
	app.update();
	let storage = app.world().resource::<HcsgStorage>();
	assert_eq!(storage.get::<ShapedCell>(id), Some(&ShapedCell(2)));
	assert!(storage.entry::<ShapedCell>(id).map(|e| e.version) > first);
	assert!(storage.contains::<ShapedCell>(Id::from_cell(cell(1.0))));
	assert_eq!(app.world().resource::<LodJobCounter>().active(), 0);
}

#[test]
fn rewriting_an_equal_root_invalidates_nothing() {
	let mut app = shaped_app();
	let id = Id::from_cell(cell(0.0));
	app.world_mut().resource_mut::<PublishNext>().0 = Some(span(0.0, 0.5));
	app.update();
	let first = app.world().resource::<HcsgStorage>().entry::<ShapedCell>(id).map(|e| e.version);

	app.world_mut().resource_mut::<Shape>().set_changed();
	app.update();
	let storage = app.world().resource::<HcsgStorage>();
	assert_eq!(storage.entry::<ShapedCell>(id).map(|e| e.version), first);
	assert!(app.world().resource::<Messages<crate::hcsg::Reseeded<Shape>>>().is_empty());
}

#[derive(Clone)]
struct Gate;

crate::seeded_root!(Gate);

/// Builds only once [`Gate`] is seeded.
struct Gated;

impl<S> GenerationScheme<S> for Gated
where
	S: GeneratingSpatialIndex<Terrain> + GeneratingSpatialIndex<Gate>,
{
	fn original_ids_for(spatial_index: &mut S, region: Aabb3d) -> Vec<OriginalId> {
		GeneratingSpatialIndex::<Terrain>::original_ids_for(spatial_index, region)
	}

	fn build_with_id(spatial_index: &mut S, id: Id) -> Option<(Self, Aabb3d)> {
		GeneratingSpatialIndex::<Gate>::get_one_or_generate(spatial_index, Id::Universal)?;
		Some((Self, id.origin_cell_bounds()?))
	}
}

#[test]
fn failed_builds_retry_on_restart() {
	let mut app = app();
	app.add_plugins(GenerateOn::<Window, Gated>::default());
	let keep = span(0.0, 0.5);
	let id = Id::from_cell(cell(0.0));
	app.world_mut().resource_mut::<PublishNext>().0 = Some(keep);
	app.update();
	assert!(!app.world().resource::<HcsgStorage>().contains::<Gated>(id));

	app.world_mut()
		.resource_mut::<HcsgStorage>()
		.seed(Gate, crate::hcsg::universal_bounds());
	app.update();
	assert!(
		!app.world().resource::<HcsgStorage>().contains::<Gated>(id),
		"a failed id is dropped, not polled"
	);

	app.world_mut().resource_mut::<RestartNext>().0 = Some(keep);
	app.update();
	assert!(app.world().resource::<HcsgStorage>().contains::<Gated>(id));
	assert_eq!(app.world().resource::<LodJobCounter>().active(), 0);
}

#[derive(Resource, Default)]
struct RescanNext(Vec<Aabb3d>);

fn rescan(mut next: ResMut<RescanNext>, mut channel: GenerationProducer<Window>) {
	if !next.0.is_empty() {
		channel.rescan(std::mem::take(&mut next.0));
	}
}

#[test]
fn rescan_retries_only_the_named_strip_of_an_unmoved_window() {
	let mut app = app();
	app.init_resource::<RescanNext>()
		.add_plugins(GenerateOn::<Window, Gated>::default())
		.add_systems(
			Update,
			rescan.in_set(crate::gen::LodGenerateSystems::Produce).after(publish),
		);
	app.world_mut().resource_mut::<PublishNext>().0 = Some(span(0.0, 1.5));
	app.update();
	app.world_mut()
		.resource_mut::<HcsgStorage>()
		.seed(Gate, crate::hcsg::universal_bounds());

	app.world_mut().resource_mut::<RescanNext>().0 = vec![span(1.25, 1.5)];
	app.update();
	let storage = app.world().resource::<HcsgStorage>();
	assert!(storage.contains::<Gated>(Id::from_cell(cell(1.0))));
	assert!(
		!storage.contains::<Gated>(Id::from_cell(cell(0.0))),
		"ids outside the rescanned strip stay as they were"
	);
	assert_eq!(app.world().resource::<LodJobCounter>().active(), 0);
}

#[test]
fn resting_drivers_keep_their_coverage_while_another_moves() {
	use crate::hcsg::{CurrentBounds, ProduceFromNodes};
	use crate::lod_ref::{LodNode, LodNodePose};
	use crate::scene::Bullseye;
	use bevy::math::bounding::BoundingVolume;

	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_plugins(ProduceFromNodes::<Bullseye, ()>::default())
		.insert_resource(Bullseye::new(10.0, 40.0));
	let at = |x: f32| LodNodePose {
		previous: Transform::from_xyz(x, 0.0, 0.0),
		current: Transform::from_xyz(x, 0.0, 0.0),
	};
	let resting = app.world_mut().spawn((LodNode, at(500.0))).id();
	let moving = app.world_mut().spawn((LodNode, at(0.0))).id();
	app.update();

	let covers = |app: &App, x: f32| {
		app.world()
			.resource::<CurrentBounds<Bullseye>>()
			.keep()
			.is_some_and(|keep| keep.contains(&Aabb3d::new(Vec3::new(x, 0.0, 0.0), Vec3::ZERO)))
	};
	assert!(covers(&app, 0.0) && covers(&app, 500.0));

	app.world_mut().entity_mut(moving).insert(LodNodePose {
		previous: Transform::from_xyz(0.0, 0.0, 0.0),
		current: Transform::from_xyz(-100.0, 0.0, 0.0),
	});
	app.update();
	assert!(covers(&app, -100.0), "the moved driver's new region is covered");
	assert!(covers(&app, 500.0), "the resting driver keeps its region");

	app.world_mut().despawn(resting);
	app.update();
	assert!(!covers(&app, 500.0), "a departed driver's region is released");
}
