use bevy::prelude::*;
use naturescapes_demo::TerrainPlugin;

fn main() {
	// Parse seed from command line or use default
	let seed = std::env::args().nth(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(12345);

	println!("Starting terrain viewer with seed: {}", seed);

	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Terrain Viewer".to_string(),
				resolution: (1280, 720).into(),
				..default()
			}),
			..default()
		}))
		.add_plugins(TerrainPlugin { seed })
		.run();
}
