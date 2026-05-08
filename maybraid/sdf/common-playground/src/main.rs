use bevy::prelude::*;
use sdf_common_playground::{PendingStartupCommand, PlaygroundCommand, SdfCommonPlaygroundPlugin};

fn main() {
	let startup = PlaygroundCommand::parse_startup_command().unwrap_or_else(|e| {
		eprintln!("{e}");
		std::process::exit(2);
	});
	if startup.is_some() {
		println!("Startup command from argv (same as in-game / text).");
	} else {
		println!("SDF playground — press / for commands (optional argv: any `PlaygroundCommand`).");
	}

	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "SDF Common Playground".into(),
				resolution: (1280, 720).into(),
				..default()
			}),
			..default()
		}))
		.insert_resource(ClearColor(Color::hsla(201.0, 0.69, 0.62, 1.0)))
		.insert_resource(PendingStartupCommand(startup))
		.add_plugins(SdfCommonPlaygroundPlugin)
		.run();
}
