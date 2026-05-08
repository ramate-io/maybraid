use bevy::prelude::*;
use sdf_common_playground::SdfCommonPlaygroundPlugin;

fn main() {
	let seed = std::env::args().nth(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(12345);

	println!("Starting sdf-common playground (seed arg reserved for future noise variants): {seed}");

	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "SDF Common Playground".into(),
				resolution: (1280, 720).into(),
				..default()
			}),
			..default()
		}))
		.add_plugins(SdfCommonPlaygroundPlugin { seed })
		.run();
}
