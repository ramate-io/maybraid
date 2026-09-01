use std::path::{Path, PathBuf};

use bevy::prelude::*;
use firing_range_playground::{FiringRangePlugin, PendingStartupCommand, PlaygroundCommand};
use maybraid_input::PadHidPlugins;

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	let startup = PlaygroundCommand::parse_startup_command().unwrap_or_else(|error| {
		eprintln!("{error}");
		std::process::exit(2);
	});
	if startup.is_some() {
		println!("Startup command from argv (same as in-game / text).");
	} else {
		println!(
			"Firing range — WASD move, mouse/stick look, Space/A jump, click/RT fire. / pause."
		);
	}

	let assets_path = assets_root();
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Maybraid Firing Range".into(),
						resolution: (1280, 720).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() })
				.with_pad_hid(),
		)
		.insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.07)))
		.insert_resource(PendingStartupCommand(startup))
		.add_plugins(FiringRangePlugin)
		.run();
}
