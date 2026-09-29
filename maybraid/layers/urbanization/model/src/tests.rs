use bevy::app::App;
use bevy::ecs::system::SystemState;
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::World;
use durham_terrain_models::{
	BaseTerrainNoise, Durham, TerrainCellLayout, TerrainConfig, TerrainEntryStore, WorldBaseTerrain,
};
use richmond_development_models::DevelopmentEntryStore;
use richmond_urbanization::UrbanizationIndex;
use terrain_layer_model::{OnTerrain, TerrainModel, TerrainView};

use crate::Urbanization;

type UrbanizedDurham = Urbanization<OnTerrain<Durham>>;

fn empty_urbanized_world() -> World {
	let mut world = World::new();
	world.insert_resource(TerrainEntryStore::default());
	world.insert_resource(TerrainCellLayout::default());
	world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(42))));
	world.insert_resource(DevelopmentEntryStore::default());
	world.insert_resource(UrbanizationIndex::default());
	world
}

#[test]
fn without_pads_urbanization_reads_the_inner_surface() -> anyhow::Result<()> {
	let mut world = empty_urbanized_world();
	let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42)).height_at(40.0, 25.0);
	let mut state = SystemState::<TerrainView<UrbanizedDurham>>::new(&mut world);
	let view = state.get(&world)?;

	assert_eq!(view.height_at(Vec2::new(40.0, 25.0)), None);
	assert_eq!(view.height_or_fallback(Vec2::new(40.0, 25.0)), base);
	assert!(view
		.cell_ids_overlapping(Aabb3d::new(Vec3::ZERO, Vec3::splat(1_000.0)))
		.is_empty());
	Ok(())
}

#[test]
#[should_panic(expected = "requires terrain_layer_model::generation::BaseTerrainGenerationPlugin")]
fn requirements_recurse_to_the_base_terrain() {
	UrbanizedDurham::require_generation(&App::new());
}
