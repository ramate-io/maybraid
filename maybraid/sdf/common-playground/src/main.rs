use bevy::prelude::*;
use sdf_common_playground::SdfCommonPlaygroundPlugin;

fn main() {
	println!("SDF playground — press / then e.g. `render tapered-cylinder` or `settings checker-size --meters 5`");

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
		.add_plugins(SdfCommonPlaygroundPlugin)
		.run();
}
