//! The world's layer stack: terrain, urbanization, vegetation, and mobs, each
//! generated and then presented.

use barking::{Barking, BarkingConfig};
use bevy::prelude::*;
use chico::{Chico, ChicoConfig};
use durham::{Durham, DurhamTerrainConfig};
use layer_stack::GenerationModePlugin;
use maybraid_game_mode_discover::Discovery;
use maybraid_game_mode_training_ground::{TrainingGround, TRAINING_FINE_HALF_EXTENT_CELLS};
use mob_layer_model::{MobGenerationPlugin, Mobs};
use mob_layer_presentation::MobPresentationPlugin;
use richmond::{Richmond, RichmondConfig};
use terrain_layer_model::{BaseTerrainGenerationPlugin, OnTerrain};
use terrain_layer_presentation::TerrainPresentationPlugin;
use urbanization_layer_model::{Urbanization, UrbanizationGenerationPlugin};
use urbanization_layer_presentation::{PaddedCells, UrbanizationPresentationPlugin};
use vegetation_layer_model::{Vegetation, VegetationGenerationPlugin};
use vegetation_layer_presentation::VegetationPresentationPlugin;

type Urbanized = Urbanization<Richmond<OnTerrain<Durham>>>;
type Forested = Chico<Urbanized>;
type Vegetated = Vegetation<Forested>;
#[allow(dead_code)]
type Inhabited = Mobs<Barking<Vegetated>>;

/// Every world layer at world defaults.
///
/// Add after [`crate::WorldMaterialRefPlugin`], which must install the world
/// material library before any layer installs its own, and after
/// `PlayerCameraPlugin`, since mob presentation adds `PlayerPlugin` when missing.
pub struct WorldLayersPlugin;

impl Plugin for WorldLayersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			(
				GenerationModePlugin::<Discovery>::initial(),
				BaseTerrainGenerationPlugin::<Discovery, Durham>::new(
					DurhamTerrainConfig::playable_world(),
				),
				UrbanizationGenerationPlugin::<Discovery, Richmond<OnTerrain<Durham>>>::new(
					RichmondConfig::world_defaults(),
				),
				VegetationGenerationPlugin::<Discovery, Forested>::new(ChicoConfig::world_defaults()),
				MobGenerationPlugin::<Discovery, Barking<Vegetated>>::new(
					BarkingConfig::world_defaults(),
				),
				TerrainPresentationPlugin::<Discovery, Urbanized, PaddedCells>::default(),
				UrbanizationPresentationPlugin::<Discovery, Richmond<OnTerrain<Durham>>>::default(),
				VegetationPresentationPlugin::<Discovery, Forested>::default(),
				MobPresentationPlugin::<Discovery, Barking<Vegetated>>::default(),
			),
			(
				GenerationModePlugin::<TrainingGround>::default(),
				BaseTerrainGenerationPlugin::<TrainingGround, Durham>::new(
					DurhamTerrainConfig::fine_patch(TRAINING_FINE_HALF_EXTENT_CELLS),
				),
				UrbanizationGenerationPlugin::<TrainingGround, Richmond<OnTerrain<Durham>>>::new(
					RichmondConfig::shared_world(),
				),
				VegetationGenerationPlugin::<TrainingGround, Forested>::new(ChicoConfig::grove()),
				MobGenerationPlugin::<TrainingGround, Barking<Vegetated>>::new(
					BarkingConfig::world_defaults(),
				),
				TerrainPresentationPlugin::<TrainingGround, Urbanized, PaddedCells>::default(),
				UrbanizationPresentationPlugin::<
					TrainingGround,
					Richmond<OnTerrain<Durham>>,
				>::default(),
				VegetationPresentationPlugin::<TrainingGround, Forested>::default(),
				MobPresentationPlugin::<TrainingGround, Barking<Vegetated>>::default(),
			),
		));
	}
}
