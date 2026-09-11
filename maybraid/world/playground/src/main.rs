use std::path::{Path, PathBuf};

use bevy::prelude::*;
use maybraid_input::PadHidPlugins;
use maybraid_world::{
	default_window_present_mode, player_spawn_xz, resolve_start_at, PendingStartupCommand,
	PlaygroundCommand, WorldPlugin,
};

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
	let (start_at, rest) = resolve_start_at(std::env::args_os().skip(1)).unwrap_or_else(|error| {
		eprintln!("{error}");
		std::process::exit(2);
	});
	let startup = PlaygroundCommand::parse_startup_from_argv_tail(rest).unwrap_or_else(|error| {
		eprintln!("{error}");
		std::process::exit(2);
	});

	let assets_path = assets_root();
	App::new()
		.insert_resource(player_spawn_xz(start_at))
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Maybraid World".into(),
						resolution: (1280, 720).into(),
						present_mode: default_window_present_mode(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() })
				.with_pad_hid(),
		)
		.insert_resource(PendingStartupCommand(startup))
		.add_plugins(WorldPlugin::default())
		.run();
}
