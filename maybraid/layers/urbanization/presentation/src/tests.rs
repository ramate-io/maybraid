use std::collections::HashSet;

use bevy::app::{App, Plugin};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::{Visibility, World};
use durham_terrain_models::{
	Durham, DurhamTerrainConfig, PresentedTerrainScene, TerrainColliderMeshSource,
	TerrainSuperseded, TerrainTrimeshCollider,
};
use lod::gen::Id;
use richmond_development_models::PresentedPaddedTerrainScene;
use terrain_layer_model::{
	BaseTerrainGenerationPlugin, BaseTerrainScheme, GenerationMode, GenerationModePlugin, OnTerrain,
};
use terrain_layer_presentation::TerrainPresentationPlugin;
use urbanization_layer_model::{
	Urbanization, UrbanizationGenerationPlugin, UrbanizationLayerConfig,
};

use crate::{
	sync_raw_terrain_replacements, PaddedCells, UrbanizationPaddedTerrainState,
	UrbanizationPresentationPlugin,
};

#[test]
fn raw_cell_hands_its_floor_to_a_cooked_padded_replacement() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let mut world = World::new();
	world.insert_resource(UrbanizationPaddedTerrainState {
		wanted: HashSet::from([id]),
		replaced: HashSet::new(),
	});
	let raw = world.spawn((PresentedTerrainScene(id), Visibility::Inherited)).id();
	let padded = world.spawn((PresentedPaddedTerrainScene(id), TerrainColliderMeshSource)).id();
	let run = |world: &mut World| {
		world
			.run_system_once(sync_raw_terrain_replacements)
			.map_err(|error| anyhow::anyhow!("{error:?}"))
	};

	run(&mut world)?;
	assert!(world.get::<TerrainSuperseded>(raw).is_none(), "uncooked pads cannot bear weight");
	assert_eq!(world.get::<Visibility>(raw), Some(&Visibility::Inherited));

	world.entity_mut(padded).insert(TerrainTrimeshCollider);
	run(&mut world)?;
	assert!(world.get::<TerrainSuperseded>(raw).is_some(), "raw floor stays under pads");
	assert_eq!(world.get::<Visibility>(raw), Some(&Visibility::Hidden));

	world.resource_mut::<UrbanizationPaddedTerrainState>().wanted.clear();
	run(&mut world)?;
	assert!(world.get::<TerrainSuperseded>(raw).is_none(), "raw cell collides again");
	assert_eq!(world.get::<Visibility>(raw), Some(&Visibility::Inherited));
	Ok(())
}

#[test]
fn raw_cells_another_owner_superseded_stay_superseded() -> anyhow::Result<()> {
	use bevy::ecs::system::RunSystemOnce;
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let mut world = World::new();
	world.init_resource::<UrbanizationPaddedTerrainState>();
	let raw = world
		.spawn((PresentedTerrainScene(id), Visibility::Hidden, TerrainSuperseded))
		.id();
	world
		.run_system_once(sync_raw_terrain_replacements)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	assert!(world.get::<TerrainSuperseded>(raw).is_some());
	assert_eq!(world.get::<Visibility>(raw), Some(&Visibility::Hidden));
	Ok(())
}

#[test]
fn padded_viewer_quant_is_stable_inside_cell() -> anyhow::Result<()> {
	use crate::padded::quantize_viewer_xz_for_test;
	assert_eq!(quantize_viewer_xz_for_test(Vec3::new(0.1, 12.0, 7.9)), (0, 0));
	assert_eq!(quantize_viewer_xz_for_test(Vec3::new(8.0, 0.0, -0.1)), (1, -1));
	Ok(())
}

struct TestMode;

impl GenerationMode for TestMode {}

impl BaseTerrainScheme<Durham> for TestMode {
	fn install(_app: &mut App, _config: &DurhamTerrainConfig) {}
}

#[test]
fn urbanization_layers_finish_with_base_generation() {
	// Adding Durham / Richmond plugins to a bare `App` needs a render world
	// (shaders, `MaterialPlugin`). Assemblers (world, playground binary) are
	// that app. `finish` without those plugins is the should-panic sibling;
	// this locks the stack types the assemblers add.
	let _stack = (
		GenerationModePlugin::<TestMode>::initial(),
		BaseTerrainGenerationPlugin::<TestMode, Durham>::new(DurhamTerrainConfig::fine_patch(2)),
		UrbanizationGenerationPlugin::<OnTerrain<Durham>>::new(UrbanizationLayerConfig::default()),
		TerrainPresentationPlugin::<Urbanization<OnTerrain<Durham>>, PaddedCells>::default(),
		UrbanizationPresentationPlugin::<Urbanization<OnTerrain<Durham>>>::default(),
	);
}

#[test]
#[should_panic(expected = "requires terrain_layer_model::generation::BaseTerrainGenerationCore")]
fn urbanization_layers_without_base_generation_name_the_missing_plugin() {
	// `build` installs Richmond / furniture plugins that need a full Bevy app.
	// `finish` is the requirement check the assemblers actually run.
	UrbanizationGenerationPlugin::<OnTerrain<Durham>>::default().finish(&mut App::new());
}
