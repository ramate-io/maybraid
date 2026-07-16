fn main() {
	use bevy::prelude::*;
	use durham_terrain_models_playground::{
		PendingStartupCommand, PlaygroundCommand, TerrainModelsPlaygroundPlugin,
	};

	let startup = PlaygroundCommand::parse_startup_command().unwrap_or_else(|e| {
		eprintln!("{e}");
		std::process::exit(2);
	});

	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Durham Terrain Models Playground".into(),
				resolution: (1280, 720).into(),
				..default()
			}),
			..default()
		}))
		.insert_resource(PendingStartupCommand(startup))
		.add_plugins(TerrainModelsPlaygroundPlugin)
		.run();
}
