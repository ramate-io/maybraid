//! Slim world-playground commands. Forest + terrain extents are baked in.

use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::commands::{
	RequestMeshStats, RequestModeCharacter, RequestModeFree,
};
use chico_vegetation_on_terrain_playground::{
	CharacterSpecies, RequestFpsToggle, RequestSetCharacter,
};
use clap::{Parser, Subcommand};
use game_commands::command::{CommandScript, GameCommand};

pub const PLAYGROUND_CLI_NAME: &str = "maybraid-world";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "maybraid-world",
	version,
	about = "World model: Durham terrain, streamed forest, sky dome, character",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Switch between free-look fly camera and third-person character control.
	#[command(subcommand)]
	Mode(Mode),
	/// Replace the capsule with a Crozon character (default preview recipe).
	SetCharacter {
		species: CharacterSpecies,
	},
	#[command(subcommand)]
	Stats(Stats),
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Mode {
	/// Free-look fly camera (WASD + mouse, Space/Shift vertical).
	Free,
	/// Capsule or Crozon character with third-person camera (WASD move, Space jump).
	Character,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Stats {
	/// Mesh triangle counts plus foliage / stick / structural LOD probe hosts.
	Mesh,
	/// Toggle the FPS HUD and `[veg.timing]` log.
	Fps,
}

impl PlaygroundCommand {
	pub fn long_help_string() -> String {
		<Self as GameCommand>::long_help_string()
	}

	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_command()
	}

	pub fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			PlaygroundCommand::Help => *console = Self::long_help_string(),
			PlaygroundCommand::Script(s) => s.run(commands, console),
			PlaygroundCommand::Mode(mode) => mode.react(commands, console),
			PlaygroundCommand::SetCharacter { species } => {
				commands.spawn(RequestSetCharacter { species });
				*console = format!("set-character {}: pending", species.label());
			}
			PlaygroundCommand::Stats(stats) => stats.react(commands, console),
		}
	}

	pub fn parse_line(line: &str) -> Result<Self, String> {
		<Self as GameCommand>::parse_line(line)
	}
}

impl Mode {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Mode::Free => {
				commands.spawn(RequestModeFree);
				*console = "mode free: pending".into();
			}
			Mode::Character => {
				commands.spawn(RequestModeCharacter);
				*console = "mode character: pending".into();
			}
		}
	}
}

impl Stats {
	fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Stats::Mesh => {
				commands.spawn(RequestMeshStats);
				*console = "stats mesh: pending".into();
			}
			Stats::Fps => {
				commands.spawn(RequestFpsToggle);
				*console = "stats fps: toggling".into();
			}
		}
	}
}

impl GameCommand for PlaygroundCommand {
	const CLI_NAME: &'static str = PLAYGROUND_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_stats_mesh() {
		let cmd = PlaygroundCommand::parse_line("stats mesh").unwrap();
		assert!(matches!(cmd, PlaygroundCommand::Stats(Stats::Mesh)));
	}

	#[test]
	fn parse_mode_and_set_character() {
		let cmd = PlaygroundCommand::parse_line("mode character").unwrap();
		assert!(matches!(cmd, PlaygroundCommand::Mode(Mode::Character)));
		let set = PlaygroundCommand::parse_line("set-character braidman").unwrap();
		assert!(matches!(
			set,
			PlaygroundCommand::SetCharacter { species: CharacterSpecies::Braidman }
		));
	}
}
