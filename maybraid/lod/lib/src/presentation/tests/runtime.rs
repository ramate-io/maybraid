use anyhow::Result;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::gen::tests::test_utils::{
	cell, span, RecordingPresenter, Terrain, Vegetation, WorldIndex,
};
use crate::gen::{GeneratingSpatialIndex, Id, LodGenerated, RegionPresenter, Version};
use crate::lod_ref::{LodNode, LodNodePose, LodRef};
use crate::presentation::{
	LodPresentBudget, LodPresentCullBudget, LodPresentCullPlugin, LodPresentKeepRegion,
	LodPresentPlugin, LodPresentQueue,
};

#[derive(SystemParam)]
struct RecordingParam<'w> {
	inner: ResMut<'w, RecordingPresenter>,
}

impl RegionPresenter<Vegetation, WorldIndex> for RecordingParam<'_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		RegionPresenter::<Vegetation, WorldIndex>::presented_version(&*self.inner, id)
	}

	fn handle(&mut self, id: Id, version: Version, value: &Vegetation, lod_ref: &LodRef) {
		RegionPresenter::<Vegetation, WorldIndex>::handle(
			&mut *self.inner,
			id,
			version,
			value,
			lod_ref,
		);
	}

	fn presented_ids(&self) -> Vec<Id> {
		RegionPresenter::<Vegetation, WorldIndex>::presented_ids(&*self.inner)
	}

	fn hide(&mut self, id: Id) {
		RegionPresenter::<Vegetation, WorldIndex>::hide(&mut *self.inner, id)
	}

	fn is_hidden(&self, id: Id) -> bool {
		RegionPresenter::<Vegetation, WorldIndex>::is_hidden(&*self.inner, id)
	}

	fn remove_stale(&mut self, wanted: &std::collections::HashSet<Id>) {
		RegionPresenter::<Vegetation, WorldIndex>::remove_stale(&mut *self.inner, wanted);
	}
}

impl RegionPresenter<Terrain, WorldIndex> for RecordingParam<'_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		RegionPresenter::<Terrain, WorldIndex>::presented_version(&*self.inner, id)
	}

	fn handle(&mut self, id: Id, version: Version, value: &Terrain, lod_ref: &LodRef) {
		RegionPresenter::<Terrain, WorldIndex>::handle(
			&mut *self.inner,
			id,
			version,
			value,
			lod_ref,
		);
	}

	fn presented_ids(&self) -> Vec<Id> {
		RegionPresenter::<Terrain, WorldIndex>::presented_ids(&*self.inner)
	}

	fn hide(&mut self, id: Id) {
		RegionPresenter::<Terrain, WorldIndex>::hide(&mut *self.inner, id)
	}

	fn is_hidden(&self, id: Id) -> bool {
		RegionPresenter::<Terrain, WorldIndex>::is_hidden(&*self.inner, id)
	}

	fn remove_stale(&mut self, wanted: &std::collections::HashSet<Id>) {
		RegionPresenter::<Terrain, WorldIndex>::remove_stale(&mut *self.inner, wanted);
	}
}

#[derive(Debug, Clone, Copy, Default)]
struct PresentChan;

#[test]
fn shared_budget_rotates_across_present_types() -> Result<()> {
	let mut index = WorldIndex::default();
	let identity = Transform::IDENTITY;
	let terrain_bounds = cell(0.0);
	let vegetation_bounds = cell(2.0);
	let lod = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &terrain_bounds,
	};
	GeneratingSpatialIndex::<Terrain>::get_or_generate(
		&mut index,
		Id::from_cell(terrain_bounds),
		&lod,
	);
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(
		&mut index,
		Id::from_cell(vegetation_bounds),
		&lod,
	);

	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(index)
		.insert_resource(RecordingPresenter::default())
		.insert_resource(LodPresentBudget { ids_per_frame: 1 })
		.insert_resource({
			let mut keep = LodPresentKeepRegion::<PresentChan>::default();
			keep.region = Some(span(0.0, 4.0));
			keep
		})
		.add_plugins((
			LodPresentPlugin::<Terrain, WorldIndex, RecordingParam, PresentChan>::default(),
			LodPresentPlugin::<Vegetation, WorldIndex, RecordingParam, PresentChan>::default(),
		));
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));

	app.update();
	let presenter = app.world().resource::<RecordingPresenter>();
	assert_eq!(presenter.terrain.len() + presenter.vegetation.len(), 1);

	app.update();
	let presenter = app.world().resource::<RecordingPresenter>();
	assert_eq!(presenter.terrain.len(), 1);
	assert_eq!(presenter.vegetation.len(), 1);
	Ok(())
}

#[test]
fn drain_present_picks_up_keep_region_without_a_new_message() -> Result<()> {
	let mut app = App::new();
	let mut index = WorldIndex::default();
	let identity = Transform::IDENTITY;
	let bounds = cell(2.0);
	let lod = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(
		&mut index,
		Id::from_cell(cell(2.0)),
		&lod,
	);
	app.add_plugins(MinimalPlugins)
		.insert_resource(index)
		.insert_resource(RecordingPresenter::default())
		.insert_resource(LodPresentBudget { ids_per_frame: 1 })
		.insert_resource({
			let mut keep = LodPresentKeepRegion::<PresentChan>::default();
			keep.region = Some(cell(2.0));
			keep
		})
		.add_plugins(
			LodPresentPlugin::<Vegetation, WorldIndex, RecordingParam, PresentChan>::default(),
		);
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app.update();

	let presenter = app.world().resource::<RecordingPresenter>();
	assert!(presenter.vegetation.contains_key(&Id::from_cell(cell(2.0))));
	let _ = app.world().resource::<LodPresentQueue<Vegetation>>();
	Ok(())
}

#[test]
fn drain_present_drops_pending_outside_keep_slack() -> Result<()> {
	let mut app = App::new();
	let mut index = WorldIndex::default();
	let identity = Transform::IDENTITY;
	let near = cell(0.0);
	let far = cell(250.0);
	let lod = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &near,
	};
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, Id::from_cell(near), &lod);
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, Id::from_cell(far), &lod);
	app.add_plugins(MinimalPlugins)
		.insert_resource(index)
		.insert_resource(RecordingPresenter::default())
		.insert_resource(LodPresentBudget { ids_per_frame: 1 })
		.insert_resource({
			let mut keep = LodPresentKeepRegion::<PresentChan>::default();
			keep.region = Some(near);
			keep
		})
		.insert_resource({
			let mut queue = LodPresentQueue::<Vegetation>::default();
			queue.enqueue(Id::from_cell(far));
			queue.enqueue(Id::from_cell(near));
			queue
		})
		.add_plugins(
			LodPresentPlugin::<Vegetation, WorldIndex, RecordingParam, PresentChan>::default(),
		);
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app.update();

	let presenter = app.world().resource::<RecordingPresenter>();
	assert!(presenter.vegetation.contains_key(&Id::from_cell(near)));
	assert!(!presenter.vegetation.contains_key(&Id::from_cell(far)));
	let queue = app.world().resource::<LodPresentQueue<Vegetation>>();
	assert!(!queue.contains(&Id::from_cell(far)));
	Ok(())
}

#[test]
fn drain_present_keeps_pending_inside_tile_cross_slack() -> Result<()> {
	let mut app = App::new();
	let mut index = WorldIndex::default();
	let identity = Transform::IDENTITY;
	let edge = cell(0.0);
	let keep = cell(100.0);
	let lod = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &edge,
	};
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, Id::from_cell(edge), &lod);
	app.add_plugins(MinimalPlugins)
		.insert_resource(index)
		.insert_resource(RecordingPresenter::default())
		.insert_resource(LodPresentBudget { ids_per_frame: 1 })
		.insert_resource({
			let mut keep_r = LodPresentKeepRegion::<PresentChan>::default();
			keep_r.region = Some(keep);
			keep_r
		})
		.insert_resource({
			let mut queue = LodPresentQueue::<Vegetation>::default();
			queue.enqueue(Id::from_cell(edge));
			queue
		})
		.add_plugins(
			LodPresentPlugin::<Vegetation, WorldIndex, RecordingParam, PresentChan>::default(),
		);
	app.world_mut().spawn((
		LodNode,
		LodNodePose { current: Transform::from_xyz(100.4, 0.0, 0.0), ..default() },
		Transform::from_xyz(100.4, 0.0, 0.0),
	));
	app.update();

	let presenter = app.world().resource::<RecordingPresenter>();
	assert!(presenter.vegetation.contains_key(&Id::from_cell(edge)));
	Ok(())
}

#[test]
fn drain_present_cull_hides_leaving_id_without_a_lattice_message() -> Result<()> {
	let mut index = WorldIndex::default();
	let identity = Transform::IDENTITY;
	let near = cell(0.0);
	let far = cell(250.0);
	let lod = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &near,
	};
	let near_id = Id::from_cell(near);
	let far_id = Id::from_cell(far);
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, near_id, &lod);
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, far_id, &lod);

	let mut presenter = RecordingPresenter::default();
	RegionPresenter::<Vegetation, _>::present(&mut presenter, &index, span(0.0, 251.0), &lod);
	assert!(presenter.vegetation.contains_key(&near_id));
	assert!(presenter.vegetation.contains_key(&far_id));

	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(index)
		.insert_resource(presenter)
		.insert_resource(LodPresentCullBudget { despawns_per_frame: 0 })
		.insert_resource({
			let mut keep = LodPresentKeepRegion::<PresentChan>::default();
			keep.region = Some(near);
			keep
		})
		.add_plugins(
			LodPresentCullPlugin::<Vegetation, WorldIndex, RecordingParam, PresentChan>::default(),
		);
	app.update();

	let presenter = app.world().resource::<RecordingPresenter>();
	assert!(
		presenter.hidden.contains(&far_id),
		"drain hides leaving ids from keep without a lattice message"
	);
	assert!(presenter.vegetation.contains_key(&far_id));
	assert!(presenter.vegetation.contains_key(&near_id));
	Ok(())
}

#[test]
fn drain_present_drains_remaining_queue_without_a_keep_rescan() -> Result<()> {
	let mut app = App::new();
	let mut index = WorldIndex::default();
	let identity = Transform::IDENTITY;
	let near = cell(0.0);
	let mid = cell(2.0);
	let lod = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &near,
	};
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, Id::from_cell(near), &lod);
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, Id::from_cell(mid), &lod);
	app.add_plugins(MinimalPlugins)
		.insert_resource(index)
		.insert_resource(RecordingPresenter::default())
		.insert_resource(LodPresentBudget { ids_per_frame: 1 })
		.insert_resource({
			let mut keep = LodPresentKeepRegion::<PresentChan>::default();
			keep.region = Some(span(0.0, 4.0));
			keep
		})
		.add_plugins(
			LodPresentPlugin::<Vegetation, WorldIndex, RecordingParam, PresentChan>::default(),
		);
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app.update();
	app.update();

	let presenter = app.world().resource::<RecordingPresenter>();
	assert!(presenter.vegetation.contains_key(&Id::from_cell(near)));
	assert!(presenter.vegetation.contains_key(&Id::from_cell(mid)));
	Ok(())
}

#[test]
fn drain_present_picks_up_generated_id_without_a_region_message() -> Result<()> {
	let mut app = App::new();
	let mut index = WorldIndex::default();
	let identity = Transform::IDENTITY;
	let near = cell(0.0);
	let later = cell(2.0);
	let lod = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &near,
	};
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, Id::from_cell(near), &lod);
	app.add_plugins(MinimalPlugins)
		.insert_resource(index)
		.insert_resource(RecordingPresenter::default())
		.insert_resource(LodPresentBudget { ids_per_frame: 1 })
		.insert_resource({
			let mut keep = LodPresentKeepRegion::<PresentChan>::default();
			keep.region = Some(span(0.0, 4.0));
			keep
		})
		.add_plugins(
			LodPresentPlugin::<Vegetation, WorldIndex, RecordingParam, PresentChan>::default(),
		);
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app.update();

	{
		let mut index = app.world_mut().resource_mut::<WorldIndex>();
		GeneratingSpatialIndex::<Vegetation>::get_or_generate(
			&mut *index,
			Id::from_cell(later),
			&lod,
		);
	}
	app.world_mut()
		.write_message(LodGenerated::<Vegetation>::new(Id::from_cell(later)));
	app.update();

	let presenter = app.world().resource::<RecordingPresenter>();
	assert!(presenter.vegetation.contains_key(&Id::from_cell(later)));
	Ok(())
}
