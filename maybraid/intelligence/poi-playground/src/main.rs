use bevy::prelude::*;
use poi_playground::PoiPlaygroundPlugin;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Maybraid POI Intelligence".into(),
				resolution: (1440, 900).into(),
				..default()
			}),
			..default()
		}))
		.add_plugins(PoiPlaygroundPlugin)
		.run();
}
