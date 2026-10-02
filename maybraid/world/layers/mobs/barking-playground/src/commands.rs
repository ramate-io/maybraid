//! In-game clap commands for the mob-on-terrain playground.

use bevy::prelude::*;
use clap::{Parser, Subcommand};
use game_commands::command::{CommandScript, GameCommand};

pub const PLAYGROUND_CLI_NAME: &str = "mob-on-terrain";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "mob-on-terrain",
	version,
	about = "Short authored mobs on a Durham patch (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Switch between free-look fly camera and third-person character control.
	#[command(subcommand)]
	Mode(Mode),
	/// Spawn the short herd (default).
	Herd,
	/// Spawn the short pack.
	Pack,
	/// Spawn both a short herd and a short pack.
	Both,
	/// Spawn a short herd of Hars (same capsule/visual path as other plants).
	Hars,
	/// Spawn a short herd of Ylter.
	Ylter,
	/// Spawn a Hars herd and a Ylter herd.
	HarsYlter,
	/// Regenerate the terrain patch and respawn the current cast.
	Rebuild,
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Mode {
	/// Free-look fly camera (WASD + mouse, Space/Shift vertical).
	Free,
	/// Capsule with third-person camera (WASD move, Space jump).
	Character,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestModeFree;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestModeCharacter;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestHerd;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestPack;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestBoth;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestHars;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestYlter;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestHarsYlter;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestRebuild;

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
			PlaygroundCommand::Herd => {
				commands.spawn(RequestHerd);
				*console = "herd: pending".into();
			}
			PlaygroundCommand::Pack => {
				commands.spawn(RequestPack);
				*console = "pack: pending".into();
			}
			PlaygroundCommand::Both => {
				commands.spawn(RequestBoth);
				*console = "both: pending".into();
			}
			PlaygroundCommand::Hars => {
				commands.spawn(RequestHars);
				*console = "hars: pending".into();
			}
			PlaygroundCommand::Ylter => {
				commands.spawn(RequestYlter);
				*console = "ylter: pending".into();
			}
			PlaygroundCommand::HarsYlter => {
				commands.spawn(RequestHarsYlter);
				*console = "hars-ylter: pending".into();
			}
			PlaygroundCommand::Rebuild => {
				commands.spawn(RequestRebuild);
				*console = "rebuild: pending".into();
			}
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
	fn pack_parses_as_a_cast_switch() {
		let Ok(PlaygroundCommand::Pack) = PlaygroundCommand::parse_line("pack") else {
			panic!("expected pack command");
		};
	}

	#[test]
	fn hars_ylter_parses_as_a_cast_switch() {
		let Ok(PlaygroundCommand::Hars) = PlaygroundCommand::parse_line("hars") else {
			panic!("expected hars command");
		};
		let Ok(PlaygroundCommand::Ylter) = PlaygroundCommand::parse_line("ylter") else {
			panic!("expected ylter command");
		};
		let Ok(PlaygroundCommand::HarsYlter) = PlaygroundCommand::parse_line("hars-ylter") else {
			panic!("expected hars-ylter command");
		};
	}
}
