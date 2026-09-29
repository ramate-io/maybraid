use bevy::app::{App, Plugin};
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

#[test]
fn default_urbanization_noise_parses() -> anyhow::Result<()> {
	use crate::DEFAULT_URBANIZATION_NOISE;
	use procedural_common::noise_params_from_scalar_str;

	let noise = noise_params_from_scalar_str(DEFAULT_URBANIZATION_NOISE)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
	assert_eq!(noise.seed, 1337);
	assert!((noise.frequency - 0.0005).abs() < 1e-8);
	Ok(())
}

#[test]
fn parse_urbanization_kind_accepts_kebab() -> anyhow::Result<()> {
	use crate::parse_urbanization_kind;
	use richmond_urbanization::UrbanizationKind;

	assert_eq!(
		parse_urbanization_kind("frontier").map_err(|e| anyhow::anyhow!("{e}"))?,
		UrbanizationKind::Frontier
	);
	assert!(parse_urbanization_kind("not-a-city").is_err());
	Ok(())
}

#[test]
fn default_stream_radii_are_one_and_three_kilometres() -> anyhow::Result<()> {
	use crate::{stream_radii_m, DEFAULT_URBANIZATION_STREAM_RADIUS};
	use richmond_urbanization::{DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M};

	let (present, generate) = stream_radii_m(DEFAULT_URBANIZATION_STREAM_RADIUS);
	assert!((present - DEVELOPMENT_PRESENT_RADIUS_M).abs() < 1e-3);
	assert!((generate - DEVELOPMENT_GENERATE_RADIUS_M).abs() < 1e-3);
	Ok(())
}

#[test]
fn default_spec_matches_noise_string() -> anyhow::Result<()> {
	use crate::{
		UrbanizationStreamSpec, DEFAULT_URBANIZATION_NOISE, DEFAULT_URBANIZATION_STREAM_RADIUS,
	};
	use procedural_common::noise_params_from_scalar_str;

	let parsed = noise_params_from_scalar_str(DEFAULT_URBANIZATION_NOISE)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
	let spec = UrbanizationStreamSpec::default();
	assert_eq!(spec.noise.seed, parsed.seed);
	assert!((spec.noise.frequency - parsed.frequency).abs() < 1e-8);
	assert_eq!(spec.stream_radius, DEFAULT_URBANIZATION_STREAM_RADIUS);
	Ok(())
}

#[test]
fn world_defaults_enable_urbanization_stream_at_budget_16() {
	use crate::{UrbanizationLayerConfig, DEFAULT_URBANIZATION_STREAM_RADIUS};

	let config = UrbanizationLayerConfig::world_defaults();
	assert!(config.urbanization.is_some());
	assert_eq!(
		config.urbanization.map(|s| s.stream_radius),
		Some(DEFAULT_URBANIZATION_STREAM_RADIUS)
	);
	assert_eq!(config.generate_budget, 16);
	assert_eq!(UrbanizationLayerConfig::default().generate_budget, 8);
}

#[test]
#[should_panic(expected = "requires terrain_layer_model::generation::BaseTerrainGenerationPlugin")]
fn urbanization_generation_without_base_names_the_missing_plugin() {
	use crate::UrbanizationGenerationPlugin;
	use durham_terrain_models::Durham;
	use terrain_layer_model::OnTerrain;

	// `build` installs Richmond plugins that need a full Bevy app. `finish`
	// is the requirement check the assemblers actually run.
	UrbanizationGenerationPlugin::<OnTerrain<Durham>>::default().finish(&mut App::new());
}
