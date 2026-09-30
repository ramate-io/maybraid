//! The world's nine layer plugins satisfy every `finish` requirement headless.

use bevy::prelude::{App, AssetPlugin, MinimalPlugins};
use durham_terrain_models::{Durham, DurhamCells, DurhamTerrainConfig};
use mob_layer_model::{MobGenerationPlugin, MobLayerConfig};
use mob_layer_presentation::MobPresentationPlugin;
use terrain_layer_model::{BaseTerrainGenerationPlugin, OnTerrain};
use terrain_layer_presentation::TerrainPresentationPlugin;
use urbanization_layer_model::{
	Urbanization, UrbanizationGenerationPlugin, UrbanizationLayerConfig,
};
use urbanization_layer_presentation::{PaddedCells, UrbanizationPresentationPlugin};
use vegetation_layer_model::{VegetationGenerationPlugin, VegetationLayerConfig};
use vegetation_layer_presentation::VegetationPresentationPlugin;

#[test]
fn layered_world_stack_finishes_headless() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()));
	app.add_plugins(BaseTerrainGenerationPlugin::<Durham>::new(
		DurhamTerrainConfig::playable_world(),
	));
	app.add_plugins(UrbanizationGenerationPlugin::<OnTerrain<Durham>>::new(
		UrbanizationLayerConfig::world_defaults(),
	));
	app.add_plugins(VegetationGenerationPlugin::new(VegetationLayerConfig::world_defaults()));
	app.add_plugins(MobGenerationPlugin::<Urbanization<OnTerrain<Durham>>>::new(
		MobLayerConfig::world_defaults(),
	));
	app.add_plugins(
		TerrainPresentationPlugin::<Urbanization<OnTerrain<Durham>>, PaddedCells>::default(),
	);
	app.add_plugins(UrbanizationPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default());
	app.add_plugins(VegetationPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default());
	app.add_plugins(MobPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default());
	app.add_plugins(TerrainPresentationPlugin::<OnTerrain<Durham>, DurhamCells>::default());
	app.finish();
}
