//! `settings` clap subcommand: checker size, seed, etc.

pub mod plugin;
mod react_checker_size;
mod react_seed;
pub(crate) mod react_settings_announcer;

pub use react_checker_size::SettingsCheckerSize;
pub use react_seed::SettingsSeed;

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
	/// Spawn this announcement plus leaf entities for [`Added`]-based reactors.
	pub fn react(self, commands: &mut Commands) {
		commands.spawn(self);
		match self {
			Settings::CheckerSize { meters } => {
				commands.spawn(SettingsCheckerSize { meters });
			}
			Settings::Seed { value } => {
				commands.spawn(SettingsSeed { value });
			}
		}
	}
}
