use std::path::{Path, PathBuf};

use bevy::prelude::*;
use maybraid_input::PadHidPlugins;
use mob_on_terrain_playground::{
	MobOnTerrainPlaygroundPlugin, PendingStartupCommand, PlaygroundCommand,
};

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	let startup = PlaygroundCommand::parse_startup_command().unwrap_or_else(|error| {
		eprintln!("{error}");
		std::process::exit(2);
	});

	let assets_path = assets_root();
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Maybraid Mob on Terrain".into(),
						resolution: (1280, 720).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() })
				.with_pad_hid(),
		)
		.insert_resource(PendingStartupCommand(startup))
		.add_plugins(MobOnTerrainPlaygroundPlugin)
		.run();
}
