use bevy::prelude::*;
use maybraid_input::PadHidPlugins;
use mob_brain_playground::MobBrainPlaygroundPlugin;

fn main() {
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Maybraid Mob Brain".into(),
						resolution: (1440, 900).into(),
						..default()
					}),
					..default()
				})
				.with_pad_hid(),
		)
		.add_plugins(MobBrainPlaygroundPlugin)
		.run();
}
