use anyhow::Result;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::gen::runtime::{
	LodGenerateBudget, LodGenerateKeepRegion, LodGeneratePlugin, LodGenerateQueue,
	LodGenerateRegion, LodGenerateRegionPlugin,
};
use crate::gen::tests::test_utils::{cell, Vegetation, WorldIndex};
use crate::gen::{Id, SpatialIndex};
use crate::lod_ref::{LodNode, LodNodePose};
use crate::scene::{Bullseye, LodRefreshCorePlugin};

#[derive(Debug, Clone, Copy, Default)]
struct GenChan;

#[test]
fn generate_plugin_does_not_add_scene_core() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.add_plugins(LodGenerateRegionPlugin::<Bullseye, (), GenChan>::default())
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, GenChan>::default());
	assert!(!app.is_plugin_added::<LodRefreshCorePlugin>());
	Ok(())
}

#[test]
fn drain_generate_materializes_one_id_per_budget() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.insert_resource(LodGenerateBudget { ids_per_frame: 1 })
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, GenChan>::default())
		.add_message::<LodGenerateRegion<GenChan>>();

	let region = Aabb3d::from_min_max(Vec3::new(2.0, 0.0, 0.0), Vec3::new(4.0, 1.0, 1.0));
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app.world_mut().write_message(LodGenerateRegion::<GenChan>::new(region));
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
	assert!(!queue.pending.is_empty());
	Ok(())
}

#[test]
fn drain_generate_prefers_ids_near_the_driver() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(WorldIndex::default())
		.insert_resource(LodGenerateBudget { ids_per_frame: 1 })
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, GenChan>::default())
		.add_message::<LodGenerateRegion<GenChan>>();

	let region = Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(9.0, 1.0, 1.0));
	let at = Transform::from_xyz(8.4, 0.0, 0.0);
	app.world_mut().spawn((LodNode, LodNodePose { current: at, ..default() }, at));
	app.world_mut().write_message(LodGenerateRegion::<GenChan>::new(region));
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
		.insert_resource(LodGenerateBudget { ids_per_frame: 1 })
		.insert_resource({
			let mut keep = LodGenerateKeepRegion::<GenChan>::default();
			keep.region = Some(cell(2.0));
			keep
		})
		.add_plugins(LodGeneratePlugin::<Vegetation, WorldIndex, GenChan>::default())
		.world_mut()
		.spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app.update();

	let index = app.world().resource::<WorldIndex>();
	assert!(SpatialIndex::<Vegetation>::get(index, Id::from_cell(cell(2.0))).is_some());
	Ok(())
}
