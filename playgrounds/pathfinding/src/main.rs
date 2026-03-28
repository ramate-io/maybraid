use bevy::prelude::*;
use pathfinding_playground::PathfindingPlaygroundPlugin;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Pathfinding playground — red chases cursor (blue)".to_string(),
				resolution: (1280, 720).into(),
				..default()
			}),
			..default()
		}))
		.add_plugins(PathfindingPlaygroundPlugin)
		.run();
}
