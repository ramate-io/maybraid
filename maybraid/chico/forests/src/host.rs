//! Grove-tile [`LodScene`] host. Presentation spawns the recipe; grow runs on first chunk.

use std::sync::{Arc, OnceLock};

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;
use lod::scene::{lod_host_scene_pending, LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;

use crate::index::forest_world_sample;
use crate::{ForestGroveRecipe, ForestGroveTile};

/// Recipe-backed grove host. [`LodScene`] chunk begin grows plants once.
#[derive(Component, Clone)]
pub struct ForestGroveHost {
	pub recipe: ForestGroveRecipe,
	grown: Arc<OnceLock<ForestGroveTile>>,
}

impl ForestGroveHost {
	pub fn new(recipe: ForestGroveRecipe) -> Self {
		Self { recipe, grown: Arc::new(OnceLock::new()) }
	}

	fn tile(&self) -> &ForestGroveTile {
		self.grown.get_or_init(|| self.recipe.grow(&forest_world_sample()))
	}
}

impl LodScene for ForestGroveHost {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		match self.grown.get() {
			Some(tile) => match_forest_grove_tile!(tile, g => g.scene_lod_level(lod_ref)),
			None => LodSceneLevel::High,
		}
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		match self.grown.get() {
			Some(tile) => match_forest_grove_tile!(tile, g => g.scene_lod_status(lod_ref)),
			None => LodSceneStatus::Changed(LodSceneLevel::High),
		}
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		match self.grown.get() {
			Some(tile) => match_forest_grove_tile!(tile, g => g.scene_lod_culls(lod_ref, current)),
			None => LodSceneCulls::None,
		}
	}

	fn scene_with_level(&self, lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds())
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		match_forest_grove_tile!(self.tile(), g => g.scene_chunks_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(self.recipe.extent.min(), self.recipe.extent.max())
	}
}
