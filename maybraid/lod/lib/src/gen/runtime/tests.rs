use anyhow::Result;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::gen::runtime::{
	LodGenerateBudget, LodGenerateKeepRegion, LodGeneratePlugin, LodGenerateQueue,
	LodGenerateRegion, LodGenerateRegionPlugin, LodGenerateTimeBudget,
};
use crate::gen::tests::test_utils::{cell, Terrain, Vegetation, WorldIndex};
use crate::gen::{Id, SpatialIndex};
use crate::jobs::LodJobCounter;
use crate::lod_ref::{LodNode, LodNodePose};
use crate::scene::{Bullseye, LodRefreshCorePlugin};

#[derive(Debug, Clone, Copy, Default)]
struct TestChan;

#[derive(Debug, Clone, Copy, Default)]
struct OtherChan;

#[test]
fn generate_plugin_does_not_add_scene_core() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.add_plugins(LodGenerateRegionPlugin::<Bullseye, (), TestChan>::default())
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, TestChan>::default());
	assert!(!app.is_plugin_added::<LodRefreshCorePlugin>());
	Ok(())
}

#[test]
fn drain_generate_materializes_one_id_per_budget() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.insert_resource(LodGenerateBudget::<TestChan>::new(1))
		.insert_resource(LodGenerateTimeBudget {
			time_per_frame: std::time::Duration::ZERO,
			..default()
		})
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, TestChan>::default())
		.add_message::<LodGenerateRegion<TestChan>>();

	let region = Aabb3d::from_min_max(Vec3::new(2.0, 0.0, 0.0), Vec3::new(4.0, 1.0, 1.0));
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app.world_mut().write_message(LodGenerateRegion::<TestChan>::new(region));
	app.update();

	let index = app.world().resource::<WorldIndex>();
	let queue = app.world().resource::<LodGenerateQueue<Vegetation>>();
	let created = [
		SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(2.0))).is_some(),
		SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(3.0))).is_some(),
	]
	.into_iter()
	.filter(|v| *v)
	.count();
	assert_eq!(created, 1);
	assert!(!queue.is_empty());
	assert_eq!(app.world().resource::<LodJobCounter>().active(), queue.len() as u64);
	while !app.world().resource::<LodGenerateQueue<Vegetation>>().is_empty() {
		app.update();
	}
	assert_eq!(app.world().resource::<LodJobCounter>().active(), 0);
	Ok(())
}

#[test]
fn drain_generate_prefers_ids_near_the_driver() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.insert_resource(LodGenerateBudget::<TestChan>::new(1))
		.insert_resource(LodGenerateTimeBudget {
			time_per_frame: std::time::Duration::ZERO,
			..default()
		})
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, TestChan>::default())
		.add_message::<LodGenerateRegion<TestChan>>();

	let region = Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(9.0, 1.0, 1.0));
	let at = Transform::from_xyz(8.4, 0.0, 0.0);
	app.world_mut().spawn((LodNode, LodNodePose { current: at, ..default() }, at));
	app.world_mut().write_message(LodGenerateRegion::<TestChan>::new(region));
	app.update();

	let index = app.world().resource::<WorldIndex>();
	assert!(SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(8.0))).is_some());
	assert!(SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(0.0))).is_none());
	Ok(())
}

#[test]
fn drain_generate_picks_up_keep_region_without_a_new_message() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.insert_resource(LodGenerateBudget::<TestChan>::new(1))
		.insert_resource({
			let mut keep = LodGenerateKeepRegion::<TestChan>::default();
			keep.region = Some(cell(2.0));
			keep
		})
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, TestChan>::default())
		.world_mut()
		.spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app.update();

	let index = app.world().resource::<WorldIndex>();
	assert!(SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(2.0))).is_some());
	Ok(())
}

#[test]
fn moving_keep_region_does_not_create_scan_work_without_an_impulse() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.insert_resource(LodGenerateBudget::<TestChan>::new(8))
		.insert_resource(LodGenerateTimeBudget {
			time_per_frame: std::time::Duration::ZERO,
			..default()
		})
		.insert_resource({
			let mut keep = LodGenerateKeepRegion::<TestChan>::default();
			keep.region = Some(cell(0.0));
			keep
		})
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, TestChan>::default())
		.add_message::<LodGenerateRegion<TestChan>>();
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));

	app.update();
	app.world_mut().resource_mut::<LodGenerateKeepRegion<TestChan>>().region = Some(cell(2.0));
	app.update();
	assert!(SpatialIndex::<Vegetation>::get(
		app.world().resource::<WorldIndex>(),
		Id::from_cell(cell(2.0)),
	)
	.is_none());

	app.world_mut().write_message(LodGenerateRegion::<TestChan>::new(cell(2.0)));
	app.update();
	assert!(SpatialIndex::<Vegetation>::get(
		app.world().resource::<WorldIndex>(),
		Id::from_cell(cell(2.0)),
	)
	.is_some());
	Ok(())
}

#[test]
fn drain_generate_drops_pending_outside_keep_slack() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.insert_resource(LodGenerateBudget::<TestChan>::new(1))
		.insert_resource({
			let mut keep = LodGenerateKeepRegion::<TestChan>::default();
			keep.region = Some(cell(0.0));
			keep
		})
		.insert_resource({
			let mut queue = LodGenerateQueue::<Vegetation>::default();
			queue.enqueue(Id::from_cell(cell(250.0)));
			queue.enqueue(Id::from_cell(cell(0.0)));
			queue
		})
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, TestChan>::default())
		.world_mut()
		.spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app.update();

	let index = app.world().resource::<WorldIndex>();
	assert!(SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(0.0))).is_some());
	assert!(SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(250.0))).is_none());
	let queue = app.world().resource::<LodGenerateQueue<Vegetation>>();
	assert!(!queue.contains(&Id::from_cell(cell(250.0))));
	Ok(())
}

#[test]
fn drain_generate_keeps_pending_inside_tile_cross_slack() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.insert_resource(LodGenerateBudget::<TestChan>::new(1))
		.insert_resource({
			let mut keep = LodGenerateKeepRegion::<TestChan>::default();
			keep.region = Some(cell(100.0));
			keep
		})
		.insert_resource({
			let mut queue = LodGenerateQueue::<Vegetation>::default();
			queue.enqueue(Id::from_cell(cell(0.0)));
			queue
		})
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, TestChan>::default())
		.world_mut()
		.spawn((
			LodNode,
			LodNodePose { current: Transform::from_xyz(100.4, 0.0, 0.0), ..default() },
			Transform::from_xyz(100.4, 0.0, 0.0),
		));
	app.update();

	let still_pending = app
		.world()
		.resource::<LodGenerateQueue<Vegetation>>()
		.contains(&Id::from_cell(cell(0.0)));
	let generated = SpatialIndex::<Vegetation>::get(
		app.world().resource::<WorldIndex>(),
		Id::from_cell(cell(0.0)),
	)
	.is_some();
	assert!(
		still_pending || generated,
		"cell one tile behind keep must stay queued or generate, not expire"
	);
	Ok(())
}

#[test]
fn drain_generate_zero_slack_expires_tile_cross() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.insert_resource(LodGenerateBudget::<TestChan>::new(1))
		.insert_resource({
			let mut keep = LodGenerateKeepRegion::<TestChan>::default();
			keep.region = Some(cell(100.0));
			keep.slack_xz = 0.0;
			keep
		})
		.insert_resource({
			let mut queue = LodGenerateQueue::<Vegetation>::default();
			queue.enqueue(Id::from_cell(cell(0.0)));
			queue
		})
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, TestChan>::default())
		.world_mut()
		.spawn((
			LodNode,
			LodNodePose { current: Transform::from_xyz(100.4, 0.0, 0.0), ..default() },
			Transform::from_xyz(100.4, 0.0, 0.0),
		));
	app.update();

	assert!(!app
		.world()
		.resource::<LodGenerateQueue<Vegetation>>()
		.contains(&Id::from_cell(cell(0.0))));
	assert!(SpatialIndex::<Vegetation>::get(
		app.world().resource::<WorldIndex>(),
		Id::from_cell(cell(0.0)),
	)
	.is_none());
	Ok(())
}

#[test]
fn independent_generate_drains_each_receive_their_own_budget() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.insert_resource(LodGenerateBudget::<TestChan>::new(1))
		.insert_resource(LodGenerateBudget::<OtherChan>::new(3))
		.insert_resource(LodGenerateTimeBudget {
			time_per_frame: std::time::Duration::ZERO,
			..default()
		})
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, TestChan>::default())
		.add_plugins(LodGeneratePlugin::<Terrain, WorldIndex, OtherChan>::default())
		.add_message::<LodGenerateRegion<TestChan>>()
		.add_message::<LodGenerateRegion<OtherChan>>();

	let vegetation_region =
		Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.0, 1.0, 1.0));
	let terrain_region = Aabb3d::from_min_max(Vec3::new(10.0, 0.0, 0.0), Vec3::new(14.0, 1.0, 1.0));
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app.world_mut()
		.write_message(LodGenerateRegion::<TestChan>::new(vegetation_region));
	app.world_mut()
		.write_message(LodGenerateRegion::<OtherChan>::new(terrain_region));
	app.update();

	let index = app.world().resource::<WorldIndex>();
	let vegetation = [
		SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(0.0))).is_some(),
		SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(1.0))).is_some(),
		SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(2.0))).is_some(),
		SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(3.0))).is_some(),
	]
	.into_iter()
	.filter(|created| *created)
	.count();
	let terrain = [
		SpatialIndex::<Terrain>::get(index, Id::from_cell(cell(10.0))).is_some(),
		SpatialIndex::<Terrain>::get(index, Id::from_cell(cell(11.0))).is_some(),
		SpatialIndex::<Terrain>::get(index, Id::from_cell(cell(12.0))).is_some(),
		SpatialIndex::<Terrain>::get(index, Id::from_cell(cell(13.0))).is_some(),
	]
	.into_iter()
	.filter(|created| *created)
	.count();
	assert_eq!(vegetation, 1);
	assert_eq!(terrain, 3);
	Ok(())
}
