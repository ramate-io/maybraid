use bevy::prelude::*;
use maybraid_input::PadHidPlugins;
use personalities_playground::PersonalitiesPlaygroundPlugin;

fn main() {
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Maybraid Personalities".into(),
						resolution: (1440, 900).into(),
						..default()
					}),
					..default()
				})
				.with_pad_hid(),
		)
		.add_plugins(PersonalitiesPlaygroundPlugin)
		.run();
}
