use std::path::{Path, PathBuf};

use bevy::prelude::*;
use crozon_character_world_movements_playground::{
	CharacterSpecies, CharacterWorldMovementsPlaygroundPlugin, PendingStartupCommand,
	PlaygroundCommand,
};

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	let startup = PlaygroundCommand::parse_startup_command().unwrap_or_else(|e| {
		eprintln!("{e}");
		std::process::exit(2);
	});
	let startup =
		startup.or(Some(PlaygroundCommand::SetCharacter { species: CharacterSpecies::Braidman }));

	let assets_path = assets_root();
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Crozon Character World Movements".into(),
						resolution: (1280, 720).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() }),
		)
		.insert_resource(PendingStartupCommand(startup))
		.add_plugins(CharacterWorldMovementsPlaygroundPlugin)
		.run();
}
