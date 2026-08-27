use anyhow::Result;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::gen::tests::test_utils::{cell, RecordingPresenter, Vegetation, WorldIndex};
use crate::gen::{GeneratingSpatialIndex, Id, RegionPresenter, Version};
use crate::lod_ref::{LodNode, LodNodePose, LodRef};
use crate::presentation::{
	LodPresentKeepRegion, LodPresentSceneBudget, LodPresentScenePlugin, LodPresentSceneQueue,
	PresentationChunk, PresentationPresenter, PresentationScene,
};

impl PresentationScene<WorldIndex> for Vegetation {
	type Constituent = u32;

	fn presentation_chunks(
		&self,
		_spatial_index: &WorldIndex,
		_lod_ref: &LodRef,
	) -> PresentationChunk<u32> {
		PresentationChunk::chunks([
			PresentationChunk::primitive(1),
			PresentationChunk::primitive(2),
		])
	}
}

#[derive(Resource, Default)]
struct SceneRecording {
	inner: RecordingPresenter,
	constituents: Vec<(Id, u32)>,
	finished: Vec<Id>,
}

#[derive(SystemParam)]
struct SceneParam<'w> {
	inner: ResMut<'w, SceneRecording>,
}

impl RegionPresenter<Vegetation, WorldIndex> for SceneParam<'_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		RegionPresenter::<Vegetation, WorldIndex>::presented_version(&self.inner.inner, id)
	}

	fn handle(&mut self, id: Id, version: Version, value: &Vegetation, lod_ref: &LodRef) {
		RegionPresenter::<Vegetation, WorldIndex>::handle(
			&mut self.inner.inner,
			id,
			version,
			value,
			lod_ref,
		);
	}

	fn presented_ids(&self) -> Vec<Id> {
		RegionPresenter::<Vegetation, WorldIndex>::presented_ids(&self.inner.inner)
	}

	fn remove_stale(&mut self, wanted: &std::collections::HashSet<Id>) {
		RegionPresenter::<Vegetation, WorldIndex>::remove_stale(&mut self.inner.inner, wanted);
	}
}

impl PresentationPresenter<Vegetation, WorldIndex> for SceneParam<'_> {
	fn present_constituent(
		&mut self,
		id: Id,
		version: Version,
		constituent: u32,
		_lod_ref: &LodRef,
	) {
		self.inner.constituents.push((id, constituent));
		self.inner.inner.vegetation.insert(id, version);
	}

	fn finish_presentation(&mut self, id: Id, version: Version) {
		self.inner.finished.push(id);
		self.inner.inner.vegetation.insert(id, version);
	}
}

#[derive(Debug, Clone, Copy, Default)]
struct PresentChan;

#[test]
fn scene_fulfill_drains_one_constituent_per_frame() -> Result<()> {
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
		.insert_resource(SceneRecording::default())
		.insert_resource(LodPresentSceneBudget { constituents_per_frame: 1 })
		.insert_resource({
			let mut keep = LodPresentKeepRegion::<PresentChan>::default();
			keep.region = Some(cell(2.0));
			keep
		})
		.add_plugins(
			LodPresentScenePlugin::<Vegetation, WorldIndex, SceneParam, PresentChan>::default(),
		);
	app.world_mut().spawn((LodNode, LodNodePose::default(), Transform::IDENTITY));
	app.update();
	assert_eq!(app.world().resource::<SceneRecording>().constituents.len(), 1);
	app.update();
	assert_eq!(app.world().resource::<SceneRecording>().constituents.len(), 2);
	app.update();
	assert_eq!(app.world().resource::<SceneRecording>().finished.len(), 1);
	let _ = app.world().resource::<LodPresentSceneQueue<u32>>();
	Ok(())
}
