use std::path::{Path, PathBuf};

use bevy::prelude::*;
use maybraid_input::PadHidPlugins;
use menu_playground::commands::Show;
use menu_playground::{MenuPlaygroundPlugin, PendingStartupCommand, PlaygroundCommand};

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	let startup = PlaygroundCommand::parse_startup_command().unwrap_or_else(|e| {
		eprintln!("{e}");
		std::process::exit(2);
	});
	let startup = startup.or(Some(PlaygroundCommand::Show(Show::Home)));

	let assets_path = assets_root();
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Maybraid Menu Playground".into(),
						resolution: (1280, 720).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() })
				.with_pad_hid(),
		)
		.insert_resource(ClearColor(Color::srgb(0.08, 0.10, 0.14)))
		.insert_resource(PendingStartupCommand(startup))
		.add_plugins(MenuPlaygroundPlugin)
		.run();
}
