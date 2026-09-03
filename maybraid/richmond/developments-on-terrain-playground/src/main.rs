use std::path::{Path, PathBuf};

use bevy::prelude::*;
use richmond_developments_on_terrain_playground::{
	DevelopmentsOnTerrainPlugin, PendingStartupCommand, PlaygroundCommand,
};

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	let startup = PlaygroundCommand::parse_startup_command().unwrap_or_else(|e| {
		eprintln!("{e}");
		std::process::exit(2);
	});

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
		.insert_resource(PendingStartupCommand(startup))
		.add_plugins(DevelopmentsOnTerrainPlugin::default())
		.run();
}
