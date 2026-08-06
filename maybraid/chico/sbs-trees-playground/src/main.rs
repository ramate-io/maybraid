use std::path::{Path, PathBuf};

use bevy::prelude::*;
use chico_sbs_trees_playground::{
	PendingStartupCommand, PlaygroundCommand, SbsTreesPlaygroundPlugin,
};

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	let startup = PlaygroundCommand::parse_startup_command().unwrap_or_else(|e| {
		eprintln!("{e}");
		std::process::exit(2);
	});
	if startup.is_some() {
		println!("Startup command from argv (same as in-game / text).");
	} else {
		println!("SBS trees playground — press / for commands.");
	}

	let assets_path = assets_root();
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Chico SBS Trees Playground".into(),
						resolution: (1280, 720).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() }),
		)
		.insert_resource(ClearColor(Color::srgb(0.82, 0.88, 0.92)))
		.insert_resource(PendingStartupCommand(startup))
		.add_plugins(SbsTreesPlaygroundPlugin)
		.run();
}
