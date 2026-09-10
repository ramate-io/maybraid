use bevy::prelude::*;
use clap::Parser;
use maybraid::{assets_root, GamePlugin};
use maybraid_input::PadHidPlugins;
use maybraid_world::{
	default_window_present_mode, parse_xz_metres, player_spawn_xz, start_at_from_env,
};

#[derive(Parser, Debug)]
#[command(name = "maybraid", about = "Maybraid")]
struct GameArgs {
	/// Start Discovery at world XZ metres (`x,z` or `x,y,z`; y is ignored).
	#[arg(long = "start-at", value_name = "X,Z")]
	start_at: Option<String>,
}

fn main() {
	let args = GameArgs::parse();
	let start_at = match args.start_at {
		Some(raw) => Some(parse_start_at(&raw)),
		None => start_at_from_env().unwrap_or_else(|error| {
			eprintln!("{error}");
			std::process::exit(2);
		}),
	};

	let assets_path = assets_root();
	App::new()
		.insert_resource(player_spawn_xz(start_at))
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Maybraid".into(),
						resolution: (1280, 720).into(),
						present_mode: default_window_present_mode(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() })
				.with_pad_hid(),
		)
		.add_plugins(GamePlugin)
		.run();
}

fn parse_start_at(raw: &str) -> bevy::math::Vec2 {
	parse_xz_metres(raw).unwrap_or_else(|error| {
		eprintln!("{error}");
		std::process::exit(2);
	})
}
