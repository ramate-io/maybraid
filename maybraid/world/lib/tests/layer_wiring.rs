//! The layered world stack type-checks as one `Plugins` tuple.
//!
//! Named, not added: unfinished layer builds stay `todo!()` until their issue lands.
//! Raw Durham presentation is live
//! (`TerrainPresentationPlugin::<OnTerrain<Durham>, DurhamCells>`).
//! Padded urbanized presentation is live
//! (`TerrainPresentationPlugin::<Urbanization<OnTerrain<Durham>>, PaddedCells>`).
//! Once the world assembles this stack, this test can go.

use bevy::app::Plugins;
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

fn is_plugin_stack<M>(_: impl Plugins<M>) {}

#[test]
fn layered_world_stack_type_checks() {
	is_plugin_stack((
		BaseTerrainGenerationPlugin::<Durham>::new(DurhamTerrainConfig::playable_world()),
		UrbanizationGenerationPlugin::<OnTerrain<Durham>>::new(
			UrbanizationLayerConfig::world_defaults(),
		),
		VegetationGenerationPlugin::new(VegetationLayerConfig::world_defaults()),
		MobGenerationPlugin::<Urbanization<OnTerrain<Durham>>>::new(
			MobLayerConfig::world_defaults(),
		),
		TerrainPresentationPlugin::<Urbanization<OnTerrain<Durham>>, PaddedCells>::default(),
		UrbanizationPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
		VegetationPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
		MobPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
		// Training only: raw Durham, gated off in the open world.
		TerrainPresentationPlugin::<OnTerrain<Durham>, DurhamCells>::default(),
	));
}
