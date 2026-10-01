//! The world's layer stack: terrain, urbanization, vegetation, and mobs, each
//! generated and then presented.

use bevy::prelude::*;
use durham_terrain_models::{Durham, DurhamCells, DurhamTerrainConfig};
use maybraid_game_mode_discover::Discovery;
use maybraid_game_mode_training_ground::{TrainingGround, TRAINING_FINE_HALF_EXTENT_CELLS};
use mob_layer_model::{MobGenerationPlugin, MobLayerConfig};
use mob_layer_presentation::MobPresentationPlugin;
use terrain_layer_model::{BaseTerrainGenerationPlugin, GenerationModePlugin, OnTerrain};
use terrain_layer_presentation::TerrainPresentationPlugin;
use urbanization_layer_model::{
	Urbanization, UrbanizationGenerationPlugin, UrbanizationLayerConfig,
};
use urbanization_layer_presentation::{PaddedCells, UrbanizationPresentationPlugin};
use vegetation_layer_model::{VegetationGenerationPlugin, VegetationLayerConfig};
use vegetation_layer_presentation::VegetationPresentationPlugin;

/// Every world layer at world defaults.
///
/// Add after [`crate::WorldMaterialRefPlugin`], which must install the world
/// material library before any layer installs its own, and after
/// `PlayerCameraPlugin`, since mob presentation adds `PlayerPlugin` when missing.
pub struct WorldLayersPlugin;

impl Plugin for WorldLayersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			GenerationModePlugin::<Discovery>::initial(),
			GenerationModePlugin::<TrainingGround>::default(),
			BaseTerrainGenerationPlugin::<Discovery, Durham>::new(
				DurhamTerrainConfig::playable_world(),
			),
			BaseTerrainGenerationPlugin::<TrainingGround, Durham>::new(
				DurhamTerrainConfig::fine_patch(TRAINING_FINE_HALF_EXTENT_CELLS),
			),
			// Training only: raw Durham, gated off in the open world.
			TerrainPresentationPlugin::<OnTerrain<Durham>, DurhamCells>::default(),
			UrbanizationGenerationPlugin::<OnTerrain<Durham>>::new(
				UrbanizationLayerConfig::world_defaults(),
			),
			TerrainPresentationPlugin::<Urbanization<OnTerrain<Durham>>, PaddedCells>::default(),
			UrbanizationPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
			VegetationGenerationPlugin::new(VegetationLayerConfig::world_defaults()),
			VegetationPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
			MobGenerationPlugin::<Urbanization<OnTerrain<Durham>>>::new(
				MobLayerConfig::world_defaults(),
			),
			MobPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
		));
	}
}
