//! The layered world stack type-checks as one `Plugins` tuple.
//!
//! Named, not added: layer builds stay `todo!()` until each layer issue lands.
//! Once `playable_world()` assembles this stack, this test can go.

use bevy::app::Plugins;
use durham_terrain_models::{Durham, DurhamTerrainConfig};
use mob_layer_model::MobGenerationPlugin;
use mob_layer_presentation::MobPresentationPlugin;
use terrain_layer_model::{BaseTerrainGenerationPlugin, OnTerrain};
use terrain_layer_presentation::TerrainPresentationPlugin;
use urbanization_layer_model::{Urbanization, UrbanizationGenerationPlugin};
use urbanization_layer_presentation::UrbanizationPresentationPlugin;
use vegetation_layer_model::VegetationGenerationPlugin;
use vegetation_layer_presentation::VegetationPresentationPlugin;

type Ground = Urbanization<OnTerrain<Durham>>;

fn is_plugin_stack<M>(_: impl Plugins<M>) {}

#[test]
fn layered_world_stack_type_checks() {
	is_plugin_stack((
		BaseTerrainGenerationPlugin::<Durham>::new(DurhamTerrainConfig::playable_world()),
		UrbanizationGenerationPlugin::<OnTerrain<Durham>>::default(),
		VegetationGenerationPlugin,
		MobGenerationPlugin::<Ground>::default(),
		TerrainPresentationPlugin::<Ground>::default(),
		UrbanizationPresentationPlugin::<Ground>::default(),
		VegetationPresentationPlugin::<Ground>::default(),
		MobPresentationPlugin::<Ground>::default(),
	));
}
