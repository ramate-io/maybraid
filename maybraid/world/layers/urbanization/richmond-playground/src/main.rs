use std::path::{Path, PathBuf};

use bevy::prelude::*;
use durham::{Durham, DurhamCells, DurhamTerrainConfig};
use richmond_playground::{
	DevelopmentsOnTerrainPlugin, PendingStartupCommand, PlaygroundCommand, PlaygroundConfig,
	PlaygroundMode,
};
use terrain_layer_model::{BaseTerrainGenerationPlugin, OnTerrain};
use layer_stack::{GenerationModePlugin};
use terrain_layer_presentation::TerrainPresentationPlugin;
use richmond::{Richmond, RichmondConfig, UrbanizationStreamSpec};
use urbanization_layer_model::{Urbanization, UrbanizationGenerationPlugin};
use urbanization_layer_presentation::{PaddedCells, UrbanizationPresentationPlugin};

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../assets")
}

fn main() {
	let startup = PlaygroundCommand::parse_startup().unwrap_or_else(|e| {
		eprintln!("{e}");
		std::process::exit(2);
	});
	let playground = PlaygroundConfig::default();
	let urban = RichmondConfig {
		focus_development: startup.focus_development,
		urbanization: Some(UrbanizationStreamSpec::default()),
		..RichmondConfig::default()
	};

	let assets_path = assets_root();
	let mut app = App::new();
	app.add_plugins(
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
	);
	app.insert_resource(PendingStartupCommand(startup.command));
	app.add_plugins(GenerationModePlugin::<PlaygroundMode>::initial());
	app.add_plugins(BaseTerrainGenerationPlugin::<PlaygroundMode, Durham>::new(
		DurhamTerrainConfig::fine_patch(playground.terrain_radius),
	));
	app.add_plugins(
		TerrainPresentationPlugin::<PlaygroundMode, OnTerrain<Durham>, DurhamCells>::default(),
	);
	app.add_plugins(UrbanizationGenerationPlugin::<PlaygroundMode, Richmond<OnTerrain<Durham>>>::new(
		urban,
	));
	app.add_plugins(TerrainPresentationPlugin::<
		PlaygroundMode,
		Urbanization<Richmond<OnTerrain<Durham>>>,
		PaddedCells,
	>::default());
	app.add_plugins(
		UrbanizationPresentationPlugin::<PlaygroundMode, Richmond<OnTerrain<Durham>>>::default(),
	);
	app.add_plugins(DevelopmentsOnTerrainPlugin { config: playground, commands: true })
		.run();
}
