use bevy::prelude::*;
use mob_brain_playground::MobBrainPlaygroundPlugin;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Maybraid Mob Brain".into(),
				resolution: (1440, 900).into(),
				..default()
			}),
			..default()
		}))
		.add_plugins(MobBrainPlaygroundPlugin)
		.run();
}
