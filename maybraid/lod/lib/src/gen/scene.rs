use crate::gen::generation::MaterializeStatus;
use crate::gen::id::Id;
use crate::lod_ref::LodRef;
use bevy::{math::bounding::Aabb3d, scene::Scene};
use std::collections::HashSet;
use std::marker::PhantomData;

/// The asset's local opinion about whether its scene changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScenePatchStatus {
	Changed,
	Unchanged,
}

pub trait LodScene {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static;

	fn scene_patch_status(&self, lod_ref: &LodRef) -> ScenePatchStatus;
}

pub trait SceneSpawner<T: LodScene> {
	fn spawn_or_patch_scene(
		&mut self,
		id: Id,
		materialize_status: MaterializeStatus,
		scene_status: ScenePatchStatus,
		scene: impl Scene,
		marker: PhantomData<T>,
	);

	fn heal_region(&mut self, region: Aabb3d, wanted: &HashSet<Id>, marker: PhantomData<T>);
}
