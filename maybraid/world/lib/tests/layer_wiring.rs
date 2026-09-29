//! The layered world stack type-checks as one `Plugins` tuple.
//!
//! Named, not added: unfinished layer builds stay `todo!()` until their issue lands.
//! Raw Durham presentation is live (`TerrainPresentationPlugin::<DurhamCellPresenter>`).
//! Once the world assembles this stack, this test can go.

use bevy::app::Plugins;
use durham_terrain_models::{Durham, DurhamCellPresenter, DurhamTerrainConfig};
use mob_layer_model::MobGenerationPlugin;
use mob_layer_presentation::MobPresentationPlugin;
use terrain_layer_model::{BaseTerrainGenerationPlugin, OnTerrain};
use terrain_layer_presentation::TerrainPresentationPlugin;
use urbanization_layer_model::{Urbanization, UrbanizationGenerationPlugin};
use urbanization_layer_presentation::UrbanizationPresentationPlugin;
use vegetation_layer_model::VegetationGenerationPlugin;
use vegetation_layer_presentation::VegetationPresentationPlugin;

fn is_plugin_stack<M>(_: impl Plugins<M>) {}

#[test]
fn layered_world_stack_type_checks() {
	is_plugin_stack((
		BaseTerrainGenerationPlugin::<Durham>::new(DurhamTerrainConfig::playable_world()),
		TerrainPresentationPlugin::<DurhamCellPresenter>::default(),
		UrbanizationGenerationPlugin::<OnTerrain<Durham>>::default(),
		VegetationGenerationPlugin,
		MobGenerationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
		// #886 adds TerrainPresentationPlugin::<PaddedCellPresenter<OnTerrain<Durham>>>
		UrbanizationPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
		VegetationPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
		MobPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
	));
}
