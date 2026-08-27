use anyhow::Result;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::gen::tests::test_utils::{cell, RecordingPresenter, Vegetation, WorldIndex};
use crate::gen::{GeneratingSpatialIndex, Id, RegionPresenter, Version};
use crate::lod_ref::{LodNode, LodNodePose, LodRef};
use crate::presentation::{
	LodPresentBudget, LodPresentKeepRegion, LodPresentPlugin, LodPresentQueue,
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

	fn remove_stale(&mut self, wanted: &std::collections::HashSet<Id>) {
		RegionPresenter::<Vegetation, WorldIndex>::remove_stale(&mut *self.inner, wanted);
	}
}

#[derive(Debug, Clone, Copy, Default)]
struct PresentChan;

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
