use std::path::{Path, PathBuf};

use bevy::prelude::*;
use durham_terrain_models::{Durham, DurhamCells, DurhamTerrainConfig};
use richmond_developments_on_terrain_playground::{
	DevelopmentsOnTerrainPlugin, PendingStartupCommand, PlaygroundCommand, PlaygroundConfig,
	PlaygroundMode,
};
use terrain_layer_model::{
	BaseTerrainGenerationPlugin, GenerationModePlugin, OnTerrain,
};
use terrain_layer_presentation::TerrainPresentationPlugin;
use urbanization_layer_model::{
	Urbanization, UrbanizationGenerationPlugin, UrbanizationLayerConfig, UrbanizationStreamSpec,
};
use urbanization_layer_presentation::{PaddedCells, UrbanizationPresentationPlugin};

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	let startup = PlaygroundCommand::parse_startup().unwrap_or_else(|e| {
		eprintln!("{e}");
		std::process::exit(2);
	});
	let playground = PlaygroundConfig::default();
	let urban = UrbanizationLayerConfig {
		focus_development: startup.focus_development,
		urbanization: Some(UrbanizationStreamSpec::default()),
		..UrbanizationLayerConfig::default()
	};

	let assets_path = assets_root();
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Richmond Developments on Terrain".into(),
						resolution: (1280, 720).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() }),
		)
		.insert_resource(PendingStartupCommand(startup.command))
		.add_plugins(GenerationModePlugin::<PlaygroundMode>::initial())
		.add_plugins(BaseTerrainGenerationPlugin::<PlaygroundMode, Durham>::new(
			DurhamTerrainConfig::fine_patch(playground.terrain_radius),
		))
		.add_plugins(
			TerrainPresentationPlugin::<PlaygroundMode, OnTerrain<Durham>, DurhamCells>::default(),
		)
		.add_plugins(UrbanizationGenerationPlugin::<PlaygroundMode, OnTerrain<Durham>>::new(urban))
		.add_plugins(TerrainPresentationPlugin::<
			PlaygroundMode,
			Urbanization<OnTerrain<Durham>>,
			PaddedCells,
		>::default())
		.add_plugins(
			UrbanizationPresentationPlugin::<PlaygroundMode, Urbanization<OnTerrain<Durham>>>::default(
			),
		)
		.add_plugins(DevelopmentsOnTerrainPlugin { config: playground, commands: true })
		.run();
}
