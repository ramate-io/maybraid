//! The world's layer stack: terrain, urbanization, vegetation, and mobs, each
//! generated and then presented.

use bevy::prelude::*;
use durham::{Durham, DurhamTerrainConfig};
use maybraid_game_mode_discover::Discovery;
use maybraid_game_mode_training_ground::{TrainingGround, TRAINING_FINE_HALF_EXTENT_CELLS};
use mob_layer_model::{MobGenerationPlugin, MobLayerConfig};
use mob_layer_presentation::MobPresentationPlugin;
use richmond::{Richmond, RichmondConfig};
use terrain_layer_model::{BaseTerrainGenerationPlugin, OnTerrain};
use layer_stack::GenerationModePlugin;
use terrain_layer_presentation::TerrainPresentationPlugin;
use urbanization_layer_model::{Urbanization, UrbanizationGenerationPlugin};
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
		furniture_assemblies::install_on_urbanization(app);
		app.add_plugins((
			(
				GenerationModePlugin::<Discovery>::initial(),
				BaseTerrainGenerationPlugin::<Discovery, Durham>::new(
					DurhamTerrainConfig::playable_world(),
				),
				UrbanizationGenerationPlugin::<Discovery, Richmond<OnTerrain<Durham>>>::new(
					RichmondConfig::world_defaults(),
				),
				VegetationGenerationPlugin::<
					Discovery,
					Urbanization<Richmond<OnTerrain<Durham>>>,
				>::new(VegetationLayerConfig::world_defaults()),
				MobGenerationPlugin::<Discovery, Urbanization<Richmond<OnTerrain<Durham>>>>::new(
					MobLayerConfig::world_defaults(),
				),
				TerrainPresentationPlugin::<
					Discovery,
					Urbanization<Richmond<OnTerrain<Durham>>>,
					PaddedCells,
				>::default(),
				UrbanizationPresentationPlugin::<Discovery, Richmond<OnTerrain<Durham>>>::default(),
				VegetationPresentationPlugin::<
					Discovery,
					Urbanization<Richmond<OnTerrain<Durham>>>,
				>::default(),
				MobPresentationPlugin::<
					Discovery,
					Urbanization<Richmond<OnTerrain<Durham>>>,
				>::default(),
			),
			(
				GenerationModePlugin::<TrainingGround>::default(),
				BaseTerrainGenerationPlugin::<TrainingGround, Durham>::new(
					DurhamTerrainConfig::fine_patch(TRAINING_FINE_HALF_EXTENT_CELLS),
				),
				UrbanizationGenerationPlugin::<TrainingGround, Richmond<OnTerrain<Durham>>>::new(
					RichmondConfig::shared_world(),
				),
				VegetationGenerationPlugin::<
					TrainingGround,
					Urbanization<Richmond<OnTerrain<Durham>>>,
				>::new(VegetationLayerConfig::grove()),
				MobGenerationPlugin::<
					TrainingGround,
					Urbanization<Richmond<OnTerrain<Durham>>>,
				>::new(MobLayerConfig::world_defaults()),
				TerrainPresentationPlugin::<
					TrainingGround,
					Urbanization<Richmond<OnTerrain<Durham>>>,
					PaddedCells,
				>::default(),
				UrbanizationPresentationPlugin::<
					TrainingGround,
					Richmond<OnTerrain<Durham>>,
				>::default(),
				VegetationPresentationPlugin::<
					TrainingGround,
					Urbanization<Richmond<OnTerrain<Durham>>>,
				>::default(),
				MobPresentationPlugin::<
					TrainingGround,
					Urbanization<Richmond<OnTerrain<Durham>>>,
				>::default(),
			),
		));
	}
}
