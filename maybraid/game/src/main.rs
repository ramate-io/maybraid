use bevy::prelude::*;
use maybraid::{assets_root, GamePlugin};
use maybraid_input::PadHidPlugins;
use maybraid_world::default_window_present_mode;

fn main() {
	let assets_path = assets_root();
	App::new()
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
