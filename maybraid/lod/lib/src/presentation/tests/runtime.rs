use anyhow::Result;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::gen::tests::test_utils::{
	cell, span, PresenterOp, RecordingPresenter, Terrain, Vegetation, WorldIndex,
};
use crate::gen::{GeneratingSpatialIndex, Id, LodGenerated, RegionPresenter, Version};
use crate::lod_ref::{LodNode, LodNodePose, LodRef};
use crate::jobs::LodJobCounter;
use crate::presentation::{
	LodPresentBudget, LodPresentCullBudget, LodPresentCullPlugin, LodPresentGate,
	LodPresentKeepRegion, LodPresentPlugin, LodPresentQueue, LodPresentRegion,
	LodPresentTimeBudget,
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
fn independent_present_drains_each_receive_the_configured_budget() -> Result<()> {
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
		.insert_resource(LodPresentBudget::<PresentChan>::new(1))
		.insert_resource(LodPresentTimeBudget {
			time_per_frame: std::time::Duration::ZERO,
			..default()
		})
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
	assert_eq!(presenter.terrain.len(), 1);
	assert_eq!(presenter.vegetation.len(), 1);
	Ok(())
}

fn handle_count(presenter: &RecordingPresenter, id: Id) -> usize {
	presenter.ops.iter().filter(|op| matches!(op, PresenterOp::Handle(got, _) if *got == id)).count()
}

fn pending_present_app(ids: &[Id], budget: u32) -> App {
	let mut index = WorldIndex::default();
	let identity = Transform::IDENTITY;
	let bounds = cell(0.0);
	let lod = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	for id in ids {
		GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, *id, &lod);
	}
	let mut keep = LodPresentKeepRegion::<PresentChan>::default();
	keep.region = Some(span(-10.0, 20.0));
	let mut presenter = RecordingPresenter::default();
	presenter.hold_ids.extend(ids.iter().copied());
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(index)
		.insert_resource(presenter)
		.insert_resource(LodPresentBudget::<PresentChan>::new(budget))
		.insert_resource(LodPresentTimeBudget {
			time_per_frame: std::time::Duration::ZERO,
			..default()
		})
		.insert_resource(keep)
		.add_plugins(
			LodPresentPlugin::<Vegetation, WorldIndex, RecordingParam, PresentChan>::default(),
		);
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app
}

#[test]
fn incomplete_present_is_handled_once_per_drain() -> Result<()> {
	let id = Id::from_cell(cell(0.0));
	let mut app = pending_present_app(&[id], 4);
	app.update();
	let presenter = app.world().resource::<RecordingPresenter>();
	assert_eq!(handle_count(presenter, id), 1, "a larger budget must not re-poll the same id");
	Ok(())
}

#[test]
fn incomplete_present_handles_each_id_once_when_budget_exceeds_queue() -> Result<()> {
	let first = Id::from_cell(cell(0.0));
	let second = Id::from_cell(cell(2.0));
	let mut app = pending_present_app(&[first, second], 4);
	app.update();
	let presenter = app.world().resource::<RecordingPresenter>();
	assert_eq!(handle_count(presenter, first), 1);
	assert_eq!(handle_count(presenter, second), 1);
	Ok(())
}

#[test]
fn incomplete_present_yields_to_other_ids() -> Result<()> {
	let first = Id::from_cell(cell(0.0));
	let second = Id::from_cell(cell(2.0));
	let mut app = pending_present_app(&[first, second], 1);
	app.update();
	assert_eq!(handle_count(app.world().resource::<RecordingPresenter>(), first), 1);
	assert_eq!(handle_count(app.world().resource::<RecordingPresenter>(), second), 0);
	app.update();
	let presenter = app.world().resource::<RecordingPresenter>();
	assert_eq!(handle_count(presenter, first), 1, "front-of-queue retry would handle first again");
	assert_eq!(handle_count(presenter, second), 1, "back-of-queue lets the other id run");
	Ok(())
}

#[test]
fn incomplete_present_yields_when_closer_ids_keep_arriving() -> Result<()> {
	let first = Id::from_cell(cell(0.0));
	let second = Id::from_cell(cell(2.0));
	let arriving = Id::from_cell(cell(0.5));
	let mut app = pending_present_app(&[first, second], 1);
	app.update();
	assert_eq!(handle_count(app.world().resource::<RecordingPresenter>(), first), 1);
	assert_eq!(handle_count(app.world().resource::<RecordingPresenter>(), second), 0);

	let identity = Transform::IDENTITY;
	let bounds = cell(0.0);
	let lod = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	{
		let mut index = app.world_mut().resource_mut::<WorldIndex>();
		GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut *index, arriving, &lod);
	}
	app.world_mut().resource_mut::<RecordingPresenter>().hold_ids.insert(arriving);
	app.world_mut().write_message(LodGenerated::<Vegetation>::new(arriving));
	app.update();

	let presenter = app.world().resource::<RecordingPresenter>();
	assert_eq!(
		handle_count(presenter, first),
		1,
		"a closer generated id must not re-sort the incomplete front id back to drain"
	);
	assert_eq!(
		handle_count(presenter, second),
		1,
		"already-queued work must run before newly arriving ids"
	);
	assert_eq!(handle_count(presenter, arriving), 0);
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
		.insert_resource(LodPresentBudget::<PresentChan>::new(1))
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
fn moving_keep_region_does_not_create_present_scan_work_without_an_impulse() -> Result<()> {
	let mut app = App::new();
	let mut index = WorldIndex::default();
	let identity = Transform::IDENTITY;
	let initial = cell(0.0);
	let moved = cell(2.0);
	let lod = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &initial,
	};
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, Id::from_cell(initial), &lod);
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, Id::from_cell(moved), &lod);
	app.add_plugins(MinimalPlugins)
		.insert_resource(index)
		.insert_resource(RecordingPresenter::default())
		.insert_resource(LodPresentBudget::<PresentChan>::new(8))
		.insert_resource(LodPresentTimeBudget {
			time_per_frame: std::time::Duration::ZERO,
			..default()
		})
		.insert_resource({
			let mut keep = LodPresentKeepRegion::<PresentChan>::default();
			keep.region = Some(initial);
			keep
		})
		.add_plugins(
			LodPresentPlugin::<Vegetation, WorldIndex, RecordingParam, PresentChan>::default(),
		)
		.add_message::<LodPresentRegion<PresentChan>>();
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));

	app.update();
	app.world_mut().resource_mut::<LodPresentKeepRegion<PresentChan>>().region = Some(moved);
	app.update();
	assert!(!app
		.world()
		.resource::<RecordingPresenter>()
		.vegetation
		.contains_key(&Id::from_cell(moved)));

	app.world_mut().write_message(LodPresentRegion::<PresentChan>::new(moved));
	app.update();
	assert!(app
		.world()
		.resource::<RecordingPresenter>()
		.vegetation
		.contains_key(&Id::from_cell(moved)));
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
		.insert_resource(LodPresentBudget::<PresentChan>::new(1))
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
		.insert_resource(LodPresentBudget::<PresentChan>::new(1))
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

	app.update();
	let presenter = app.world().resource::<RecordingPresenter>();
	assert!(
		presenter.hidden.contains(&far_id),
		"unchanged keep + membership revision must keep the cached hide set"
	);
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
		.insert_resource(LodPresentBudget::<PresentChan>::new(1))
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
		.insert_resource(LodPresentBudget::<PresentChan>::new(1))
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

fn present_app_with_keep() -> (App, Id) {
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
	let id = Id::from_cell(cell(2.0));
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, id, &lod);
	app.add_plugins(MinimalPlugins)
		.insert_resource(index)
		.insert_resource(RecordingPresenter::default())
		.insert_resource(LodPresentBudget::<PresentChan>::new(1))
		.insert_resource(LodPresentCullBudget { despawns_per_frame: 8 })
		.insert_resource({
			let mut keep = LodPresentKeepRegion::<PresentChan>::default();
			keep.region = Some(cell(2.0));
			keep
		})
		.add_plugins((
			LodPresentPlugin::<Vegetation, WorldIndex, RecordingParam, PresentChan>::default(),
			LodPresentCullPlugin::<Vegetation, WorldIndex, RecordingParam, PresentChan>::default(),
		));
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	(app, id)
}

#[test]
fn closing_the_present_gate_retires_and_blocks_until_reopened() -> Result<()> {
	let (mut app, id) = present_app_with_keep();
	app.update();
	assert!(app.world().resource::<RecordingPresenter>().vegetation.contains_key(&id));

	app.world_mut().resource_mut::<LodPresentGate<PresentChan>>().open = false;
	app.update();
	assert!(
		!app.world().resource::<RecordingPresenter>().vegetation.contains_key(&id),
		"closing the gate retires presented ids"
	);
	assert!(
		app.world().resource::<LodPresentKeepRegion<PresentChan>>().region.is_none(),
		"closing clears the keep region"
	);

	app.update();
	assert!(
		!app.world().resource::<RecordingPresenter>().vegetation.contains_key(&id),
		"nothing re-presents while the gate is closed"
	);

	app.world_mut().resource_mut::<LodPresentKeepRegion<PresentChan>>().region = Some(cell(2.0));
	app.world_mut().resource_mut::<LodPresentGate<PresentChan>>().open = true;
	app.update();
	assert!(
		app.world().resource::<RecordingPresenter>().vegetation.contains_key(&id),
		"reopening presents again"
	);
	Ok(())
}

#[test]
fn closing_the_present_gate_releases_queued_job_tickets() -> Result<()> {
	let (mut app, _id) = present_app_with_keep();
	app.update();
	let leftover = Id::from_cell(cell(99.0));
	assert!(app.world_mut().resource_mut::<LodPresentQueue<Vegetation>>().enqueue(leftover));
	app.world().resource::<LodJobCounter>().begin();
	let before = app.world().resource::<LodJobCounter>().active();
	app.world_mut().resource_mut::<LodPresentGate<PresentChan>>().open = false;
	app.update();
	assert_eq!(
		app.world().resource::<LodJobCounter>().active(),
		before - 1,
		"gate close must release tickets owned by the cancelled queue"
	);
	assert!(
		app.world().resource::<LodPresentQueue<Vegetation>>().is_empty(),
		"gate close drops pending ids"
	);
	Ok(())
}

#[test]
fn removing_an_id_from_the_index_retires_it_without_an_extra_system() -> Result<()> {
	let (mut app, id) = present_app_with_keep();
	app.update();
	assert!(app.world().resource::<RecordingPresenter>().vegetation.contains_key(&id));

	app.world_mut().resource_mut::<WorldIndex>().vegetation.remove(&id);
	app.update();
	assert!(
		!app.world().resource::<RecordingPresenter>().vegetation.contains_key(&id),
		"cull retires a presented id that left the index"
	);
	Ok(())
}
