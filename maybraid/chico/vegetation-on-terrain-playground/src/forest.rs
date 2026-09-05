//! Durham-height forest stream. Grows on [`DurhamGroveSample`] so tiles sit on real ground.

use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_forests::{ChicoGrove, ForestIndex, ForestPresenterState, ForestStream};
use lod::gen::{Id, Version};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use lod_first_load::FirstLoadActivity;

use crate::camera::CameraController;
use crate::groves::{DurhamGroveSample, DurhamGroveTerrainCache};
use crate::PlaygroundConfig;

const PATCH_CAMERA_SPEED: f32 = 40.0;
const FOREST_CAMERA_SPEED: f32 = 80.0;

/// Present forest groves grown against the live Durham height field.
#[derive(SystemParam)]
pub struct DurhamForestPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, ForestPresenterState>,
	terrain: Res<'w, DurhamGroveTerrainCache>,
	activity: Option<Res<'w, FirstLoadActivity>>,
}

impl RegionPresenter<ChicoGrove, ForestIndex> for DurhamForestPresenter<'_, '_> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(&mut self, id: Id, version: Version, grove: &ChicoGrove, lod_ref: &LodRef) {
		let Some(terrain) = self.terrain.terrain.clone() else {
			return;
		};
		self.state.present_with_world(
			&mut self.commands,
			id,
			version,
			grove,
			lod_ref,
			|| DurhamGroveSample::from_terrain(terrain),
			self.activity.as_deref(),
		);
	}

	fn hide(&mut self, id: Id) {
		self.state.hide(&mut self.commands, id);
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.is_hidden(id)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented_ids()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		self.state.remove_stale(&mut self.commands, wanted);
	}

	fn cull(
		&mut self,
		spatial_index: &ForestIndex,
		keep: &HashSet<Id>,
		despawn_budget: u32,
	) -> u32 {
		self.state.cull(&mut self.commands, spatial_index, keep, despawn_budget)
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
