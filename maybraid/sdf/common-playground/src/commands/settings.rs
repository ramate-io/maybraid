use bevy::prelude::*;

/// Playground options (no “global” flags — invoked as `settings <subcommand>` from text mode).
#[derive(Debug, Clone, Copy, clap::Subcommand, Component)]
#[command(rename_all = "kebab-case")]
pub enum Settings {
	/// Ground checker square size in world meters.
	CheckerSize {
		#[arg(long)]
		meters: f32,
	},
	/// Reserved for deterministic previews / future batch modes.
	Seed {
		#[arg(long)]
		value: u32,
	},
}

impl Settings {
	/// Spawn a [`Settings`] entity for systems that react to [`Added<Settings>`](bevy::prelude::Added).
	pub fn react(self, commands: &mut Commands) {
		commands.spawn(self);
	}
}
