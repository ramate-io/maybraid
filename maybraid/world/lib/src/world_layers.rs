//! The world's layer stack: terrain, urbanization, vegetation, mobs, and
//! furnishing, each generated and then presented.

use barking::{Barking, BarkingConfig};
use bevy::prelude::*;
use chico::{Chico, ChicoConfig};
use durham::{Durham, DurhamTerrainConfig};
use furnishing_layer_model::Furnishing;
use layer_stack::{Generate, GenerationModePlugin, Present};
use maputo::Maputo;
use maybraid_game_mode_discover::Discovery;
use maybraid_game_mode_training_ground::{TrainingGround, TRAINING_FINE_HALF_EXTENT_CELLS};
use mob_layer_model::Mobs;
use richmond::{Richmond, RichmondConfig};
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;
use vegetation_layer_model::Vegetation;

type Ground = OnTerrain<Durham>;
type Urban = Urbanization<Richmond<Ground>>;
type Veg = Vegetation<Chico<Urban>>;
type Mob = Mobs<Barking<Veg>>;
type Furniture = Furnishing<Maputo<Urban>>;

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
				Generate::<Discovery, Ground>::new(DurhamTerrainConfig::playable_world()),
				Generate::<Discovery, Urban>::new(RichmondConfig::world_defaults()),
				Generate::<Discovery, Veg>::new(ChicoConfig::world_defaults()),
				Generate::<Discovery, Mob>::new(BarkingConfig::world_defaults()),
				Generate::<Discovery, Furniture>::new(()),
				Present::<Discovery, Ground>::default(),
				Present::<Discovery, Urban>::default(),
				Present::<Discovery, Veg>::default(),
				Present::<Discovery, Mob>::default(),
				Present::<Discovery, Furniture>::default(),
			),
			(
				GenerationModePlugin::<TrainingGround>::default(),
				Generate::<TrainingGround, Ground>::new(DurhamTerrainConfig::fine_patch(
					TRAINING_FINE_HALF_EXTENT_CELLS,
				)),
				Generate::<TrainingGround, Urban>::new(RichmondConfig::shared_world()),
				Generate::<TrainingGround, Veg>::new(ChicoConfig::grove()),
				Generate::<TrainingGround, Mob>::new(BarkingConfig::world_defaults()),
				Generate::<TrainingGround, Furniture>::new(()),
				Present::<TrainingGround, Ground>::default(),
				Present::<TrainingGround, Urban>::default(),
				Present::<TrainingGround, Veg>::default(),
				Present::<TrainingGround, Mob>::default(),
				Present::<TrainingGround, Furniture>::default(),
			),
		));
	}
}
