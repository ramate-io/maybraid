//! Durham-height forest stream. Grows on [`TerrainGroveSample`] so tiles sit on real ground.

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_forests::TerrainHeightSource;
use chico_groves::TerrainGroveSample;
use durham_terrain_models::{
	TerrainCellLayout, TerrainEntryStore, TerrainLodCell, TerrainLodIndex,
};
use lod::gen::GeneratingSpatialIndex;
use lod::lod_ref::LodRef;

use crate::camera::CameraController;
use crate::groves::OwnedDurhamTerrain;
use crate::{ForestStream, PlaygroundConfig, WorldBaseTerrain};

const PATCH_CAMERA_SPEED: f32 = 40.0;
const FOREST_CAMERA_SPEED: f32 = 80.0;

/// Seek overlapping Durham origin cells, then snapshot composed height for grow.
#[derive(SystemParam)]
pub struct DurhamHeight<'w> {
	lod: ResMut<'w, TerrainLodIndex>,
	store: Res<'w, TerrainEntryStore>,
	layout: Res<'w, TerrainCellLayout>,
	base: Res<'w, WorldBaseTerrain>,
}

impl TerrainHeightSource for DurhamHeight<'_> {
	fn ensure_and_sample(
		&mut self,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	) -> Option<impl chico_groves::GroveWorldSample + Clone + Send + Sync + 'static> {
		let _ = GeneratingSpatialIndex::<TerrainLodCell>::get_or_generate_region(
			&mut *self.lod,
			bounds,
			lod_ref,
		);
		Some(TerrainGroveSample::new(OwnedDurhamTerrain::new(
			self.store.height_snapshot(),
			self.layout.clone(),
			self.base.0.clone(),
		)))
	}
}

/// Enable the forest bullseyes when [`PlaygroundConfig::forest`] is set.
pub fn stream_durham_forest(
	config: Res<PlaygroundConfig>,
	mut stream: ResMut<ForestStream>,
	mut controller: Query<&mut CameraController, With<Camera3d>>,
	mut forest_camera: Local<bool>,
) {
	let spec = config.forest.as_ref();
	if spec.is_some() != *forest_camera {
		if let Ok(mut ctrl) = controller.single_mut() {
			ctrl.speed = if spec.is_some() { FOREST_CAMERA_SPEED } else { PATCH_CAMERA_SPEED };
		}
		*forest_camera = spec.is_some();
	}
	stream.0 = spec.copied();
}
