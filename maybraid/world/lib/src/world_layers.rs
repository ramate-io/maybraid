//! The world's layer stack: terrain, urbanization, vegetation, mobs,
//! furnishing, and language, each generated and then presented.

use barking::{Barking, BarkingConfig};
use bevy::prelude::*;
use chico::{Chico, ChicoConfig};
use durham::{Durham, DurhamTerrainConfig};
use furnishing_layer_model::Furnishing;
use geneva::{Geneva, LanguageConfig};
use language_layer_model::Language;
use layer_stack::{Generate, GenerationModePlugin, Present};
use maputo::Maputo;
use maybraid_game_mode_discover::Discovery;
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
type Named = Language<Geneva<Veg>>;

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
			Generate::<Discovery, Ground>::new(DurhamTerrainConfig::playable_world()),
			Generate::<Discovery, Urban>::new(RichmondConfig::world_defaults()),
			Generate::<Discovery, Veg>::new(ChicoConfig::world_defaults()),
			Generate::<Discovery, Mob>::new(BarkingConfig::world_defaults()),
			Generate::<Discovery, Furniture>::new(()),
			Generate::<Discovery, Named>::new(LanguageConfig::world_defaults()),
			Present::<Discovery, Ground>::default(),
			Present::<Discovery, Urban>::default(),
			Present::<Discovery, Veg>::default(),
			Present::<Discovery, Mob>::default(),
			Present::<Discovery, Furniture>::default(),
			Present::<Discovery, Named>::default(),
		));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use layer_stack::{Layer, Scheme};

	fn assert_scheme<M: Scheme<L>, L: Layer>() {}

	#[test]
	fn language_layer_is_subscribed_in_discovery() {
		assert_scheme::<Discovery, Named>();
	}
}
